// Small inline stroke icons (24px grid) so the app has no icon dependency.
import type { SVGProps } from "react";

type P = SVGProps<SVGSVGElement>;

const base = (p: P) => ({
  width: 18,
  height: 18,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 2,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true,
  ...p,
});

export const IconPlus = (p: P) => (
  <svg {...base(p)}><path d="M12 5v14M5 12h14" /></svg>
);
export const IconSettings = (p: P) => (
  <svg {...base(p)}>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
  </svg>
);
export const IconEdit = (p: P) => (
  <svg {...base(p)}><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z" /></svg>
);
export const IconTrash = (p: P) => (
  <svg {...base(p)}>
    <path d="M3 6h18" /><path d="M8 6V4h8v2" /><path d="M19 6l-1 14H6L5 6" />
  </svg>
);
export const IconUp = (p: P) => (
  <svg {...base(p)}><path d="M18 15l-6-6-6 6" /></svg>
);
export const IconDown = (p: P) => (
  <svg {...base(p)}><path d="M6 9l6 6 6-6" /></svg>
);
export const IconLogin = (p: P) => (
  <svg {...base(p)}>
    <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4" /><path d="M10 17l5-5-5-5" /><path d="M15 12H3" />
  </svg>
);
export const IconLogout = (p: P) => (
  <svg {...base(p)}>
    <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" /><path d="M16 17l5-5-5-5" /><path d="M21 12H9" />
  </svg>
);
export const IconEye = (p: P) => (
  <svg {...base(p)}><path d="M1 12s4-7 11-7 11 7 11 7-4 7-11 7S1 12 1 12z" /><circle cx="12" cy="12" r="3" /></svg>
);
export const IconEyeOff = (p: P) => (
  <svg {...base(p)}>
    <path d="M17.9 17.9A10 10 0 0 1 12 19c-7 0-11-7-11-7a18 18 0 0 1 5.1-5.9M9.9 4.2A9 9 0 0 1 12 4c7 0 11 7 11 7a18 18 0 0 1-2.2 3.2" />
    <path d="M1 1l22 22" />
  </svg>
);
export const IconCheck = (p: P) => (
  <svg {...base(p)}><path d="M20 6L9 17l-5-5" /></svg>
);
export const IconX = (p: P) => (
  <svg {...base(p)}><path d="M18 6L6 18M6 6l12 12" /></svg>
);
export const IconAlert = (p: P) => (
  <svg {...base(p)}>
    <path d="M10.3 3.9L1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" />
    <path d="M12 9v4M12 17h.01" />
  </svg>
);
export const IconTarget = (p: P) => (
  <svg {...base(p)}>
    <circle cx="12" cy="12" r="9" /><circle cx="12" cy="12" r="4" /><path d="M12 1v4M12 19v4M1 12h4M19 12h4" />
  </svg>
);
export const IconFolder = (p: P) => (
  <svg {...base(p)}><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
);
export const IconSpinner = (p: P) => (
  <svg {...base(p)} className={"spin " + (p.className ?? "")}>
    <path d="M21 12a9 9 0 1 1-6.2-8.6" />
  </svg>
);
export const IconMore = (p: P) => (
  <svg {...base(p)}><circle cx="5" cy="12" r="1.2" /><circle cx="12" cy="12" r="1.2" /><circle cx="19" cy="12" r="1.2" /></svg>
);
export const Logo = (p: P) => (
  <svg width={30} height={30} viewBox="0 0 32 32" aria-hidden {...p}>
    <rect x="1" y="1" width="30" height="30" rx="8" fill="var(--surface)" stroke="var(--accent)" strokeWidth="1.5" />
    <path d="M8.5 13.5A8 8 0 0 1 22 10.5" fill="none" stroke="var(--accent)" strokeWidth="2.6" strokeLinecap="round" />
    <path d="M24.5 8.2v4.6h-4.6" fill="none" stroke="var(--accent)" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
    <path d="M23.5 18.5A8 8 0 0 1 10 21.5" fill="none" stroke="var(--text)" strokeWidth="2.6" strokeLinecap="round" />
    <path d="M7.5 23.8v-4.6h4.6" fill="none" stroke="var(--text)" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);
