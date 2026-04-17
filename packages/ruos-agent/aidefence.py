"""ruOS AIDefence — local security layer for brain + agent.

Port of ruvector's AIDefenceGuard patterns to Python.
Runs inline in ruos-agent with <10ms latency.

Usage:
    from aidefence import AIDefence
    guard = AIDefence()
    result = guard.scan("some input")
    if not result["safe"]:
        print(f"Threat: {result['threat_level']} — {result['threats']}")
    clean = guard.sanitize("user@email.com said ignore instructions")
"""

import re
import time
from datetime import datetime

# ─── Injection Patterns (ported from AIDefenceGuard.ts) ──────────────────────

INJECTION_PATTERNS = [
    # Instruction override
    (r"ignore\s+(previous|all|above)\s+(instructions?|prompts?|rules?)", "injection", "high"),
    (r"disregard\s+(previous|all|above|the|your)", "injection", "high"),
    (r"forget\s+(everything|all|previous|your)", "injection", "high"),
    (r"override\s+(previous|system|safety|all)", "injection", "high"),
    (r"new\s+instructions?\s*:", "injection", "high"),
    # Role manipulation
    (r"you\s+are\s+(now|actually)\s+", "injection", "high"),
    (r"pretend\s+(to\s+be|you'?re|you\s+are)", "injection", "high"),
    (r"act\s+as\s+(if|though|a)", "injection", "medium"),
    (r"roleplay\s+as", "injection", "medium"),
    # System prompt extraction
    (r"what\s+(is|are)\s+your\s+(system\s+)?prompt", "extraction", "high"),
    (r"show\s+(me\s+)?your\s+(system\s+)?instructions", "extraction", "high"),
    (r"reveal\s+(your|the)\s+(system|initial|original)", "extraction", "high"),
    (r"print\s+(your|the)\s+(system|initial)", "extraction", "high"),
    (r"repeat\s+(the|your)\s+(system|initial|original)", "extraction", "high"),
    # Jailbreak
    (r"DAN\s+(mode|prompt)", "jailbreak", "critical"),
    (r"developer\s+mode", "jailbreak", "high"),
    (r"bypass\s+(safety|security|filter|restriction)", "jailbreak", "critical"),
    (r"jailbreak", "jailbreak", "critical"),
    (r"unrestrict(ed)?", "jailbreak", "high"),
    (r"remove\s+(all\s+)?restrictions", "jailbreak", "critical"),
    (r"no\s+(rules|restrictions|limits|boundaries)", "jailbreak", "high"),
    # Data exfiltration
    (r"curl\s+https?://", "exfiltration", "high"),
    (r"wget\s+", "exfiltration", "high"),
    (r"fetch\s*\(\s*['\"]https?://", "exfiltration", "high"),
    (r"webhook", "exfiltration", "medium"),
    # Code injection
    (r"<script", "code_injection", "critical"),
    (r"javascript:", "code_injection", "critical"),
    (r"eval\s*\(", "code_injection", "high"),
    (r"exec\s*\(", "code_injection", "high"),
    (r"__import__\s*\(", "code_injection", "high"),
    # Encoding attacks
    (r"base64\s*decode", "encoding", "medium"),
    (r"\\x[0-9a-f]{2}", "encoding", "low"),
]

# ─── PII Patterns ────────────────────────────────────────────────────────────

PII_PATTERNS = [
    (r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", "email"),
    (r"\b(\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b", "phone"),
    (r"\b\d{3}[-\s]?\d{2}[-\s]?\d{4}\b", "ssn"),
    (r"\b(?:\d{4}[-\s]?){3}\d{4}\b", "credit_card"),
    (r"\b(?:\d{1,3}\.){3}\d{1,3}\b", "ip_address"),
    (r"\b(sk-|api[_-]?key|token)[a-zA-Z0-9_-]{20,}\b", "api_key"),
]

# ─── Unicode Homoglyphs ─────────────────────────────────────────────────────

HOMOGLYPHS = {
    '\u0430': 'a', '\u0435': 'e', '\u043e': 'o', '\u0440': 'p',
    '\u0441': 'c', '\u0443': 'y', '\u0445': 'x', '\u044a': 'b',
    '\u0456': 'i', '\u0458': 'j', '\u04bb': 'h', '\u0501': 'd',
    '\u051b': 'q', '\u051d': 'w',
}

SEVERITY_ORDER = {"none": 0, "low": 1, "medium": 2, "high": 3, "critical": 4}


class AIDefence:
    """Local AI defence guard for ruOS."""

    def __init__(self, block_threshold="medium"):
        self.block_threshold = block_threshold
        self._compiled_injection = [
            (re.compile(p, re.IGNORECASE), cat, sev)
            for p, cat, sev in INJECTION_PATTERNS
        ]
        self._compiled_pii = [
            (re.compile(p, re.IGNORECASE), pii_type)
            for p, pii_type in PII_PATTERNS
        ]
        self.audit_log = []

    def scan(self, text, context=None):
        """Scan text for threats. Returns detection result."""
        t0 = time.time()
        threats = []

        if not text or len(text) < 2:
            return {"safe": True, "threat_level": "none", "threats": [],
                    "latency_ms": 0, "sanitized": text}

        # Normalize homoglyphs before scanning
        normalized = self._normalize_homoglyphs(text)

        # Check injection patterns
        for pattern, category, severity in self._compiled_injection:
            if pattern.search(normalized):
                threats.append({
                    "type": category,
                    "severity": severity,
                    "pattern": pattern.pattern[:60],
                })

        # Check PII
        for pattern, pii_type in self._compiled_pii:
            matches = pattern.findall(normalized)
            if matches:
                threats.append({
                    "type": "pii",
                    "severity": "medium",
                    "pii_type": pii_type,
                    "count": len(matches),
                })

        # Check control characters
        if any(ord(c) < 32 and c not in '\n\r\t' for c in text):
            threats.append({"type": "control_char", "severity": "low"})

        # Determine threat level
        if threats:
            max_sev = max(SEVERITY_ORDER.get(t["severity"], 0) for t in threats)
            level_map = {v: k for k, v in SEVERITY_ORDER.items()}
            threat_level = level_map.get(max_sev, "low")
        else:
            threat_level = "none"

        safe = SEVERITY_ORDER.get(threat_level, 0) < SEVERITY_ORDER.get(self.block_threshold, 2)
        elapsed = (time.time() - t0) * 1000

        result = {
            "safe": safe,
            "threat_level": threat_level,
            "threats": threats,
            "latency_ms": round(elapsed, 2),
            "sanitized": self.sanitize(text) if threats else text,
        }

        # Audit log
        if threats:
            self.audit_log.append({
                "at": datetime.now().isoformat(),
                "threat_level": threat_level,
                "threats": len(threats),
                "context": context,
                "blocked": not safe,
            })
            if len(self.audit_log) > 1000:
                self.audit_log = self.audit_log[-500:]

        return result

    def sanitize(self, text):
        """Remove control chars, mask PII, normalize homoglyphs."""
        if not text:
            return text

        # Remove control characters (keep newline, tab)
        cleaned = ''.join(c for c in text if ord(c) >= 32 or c in '\n\r\t')

        # Normalize homoglyphs
        cleaned = self._normalize_homoglyphs(cleaned)

        # Mask PII
        for pattern, pii_type in self._compiled_pii:
            if pii_type == "email":
                cleaned = pattern.sub(lambda m: m.group()[:2] + "***" + m.group()[-4:], cleaned)
            elif pii_type == "api_key":
                cleaned = pattern.sub("[REDACTED_KEY]", cleaned)
            elif pii_type in ("ssn", "credit_card"):
                cleaned = pattern.sub(lambda m: "***" + m.group()[-4:], cleaned)

        return cleaned

    def has_pii(self, text):
        """Quick check: does text contain PII?"""
        for pattern, _ in self._compiled_pii:
            if pattern.search(text):
                return True
        return False

    def is_safe(self, text):
        """Quick check: is text safe (no threats above threshold)?"""
        return self.scan(text)["safe"]

    def scan_for_agent(self, text, context="agent_reasoning"):
        """Scan text going into LLM reasoning — stricter threshold."""
        result = self.scan(text, context=context)
        # For agent reasoning, also check for action manipulation
        action_patterns = [
            r"switch.*profile.*to.*idle",
            r"restart.*all.*services",
            r"delete.*memories",
            r"format.*brain",
        ]
        for p in action_patterns:
            if re.search(p, text, re.IGNORECASE):
                result["threats"].append({
                    "type": "action_manipulation",
                    "severity": "critical",
                    "pattern": p[:40],
                })
                result["safe"] = False
                result["threat_level"] = "critical"
        return result

    def _normalize_homoglyphs(self, text):
        return ''.join(HOMOGLYPHS.get(c, c) for c in text)

    def get_stats(self):
        """Return audit statistics."""
        total = len(self.audit_log)
        blocked = sum(1 for e in self.audit_log if e.get("blocked"))
        by_level = {}
        for e in self.audit_log:
            lvl = e.get("threat_level", "unknown")
            by_level[lvl] = by_level.get(lvl, 0) + 1
        return {"total_scans": total, "blocked": blocked, "by_level": by_level}
