export type StrenghtResult = {
    label: string;
    color: string;
    pct: number;
    ok: boolean;
    score: number;
};

const LABELS = [
    { label: "Very weak",   color: "#ef4444" },
    { label: "Weak",        color: "#f97316" },
    { label: "Fair",        color: "#f59e0b" },
    { label: "Good",        color: "#84cc16" },
    { label: "Strong",      color: "#22c55e" },
    { label: "Very strong", color: "#10b981" },
    { label: "Excellent",   color: "#059669" },
];

const COMMON = /^(012|123|234|345|456|567|678|789|890|abc|qwe|asd|zxc|password|letmein|admin|welcome|monkey|dragon)/i;

export function scorePassword(pw: string): StrenghtResult {
    if (!pw) return { label: "", color: "#4a5568", pct: 0, ok: false, score: 0 };

    let s = 0;
    const len = pw.length;

    if (len >= 8)  s += 1;
    if (len >= 12) s += 1;
    if (len >= 16) s += 1;

    if (/[a-z]/.test(pw) && /[A-Z]/.test(pw)) s += 1;
    if (/\d/.test(pw))                        s += 1;
    if (/[^a-zA-Z0-9]/.test(pw))              s += 1;

    if (/^(.)\1+$/.test(pw)) s -= 2;
    if (COMMON.test(pw))     s -= 2;
    if (len < 8)             s = Math.min(s, 1);

    s = Math.max(0, Math.min(6, s));

    return {
        label: LABELS[s].label,
        color: LABELS[s].color,
        pct: (s / 6) * 100,
        ok: s >= 3,
        score: s,
    };
}
