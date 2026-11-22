pub struct HarnessDef {
  pub name: &'static str,
  pub config: &'static str,
  pub skills: &'static str,
  pub binary: &'static str,
}

pub const HARNESSES: &[HarnessDef] = &[
  HarnessDef {
    name: "cursor",
    config: ".cursor",
    skills: ".cursor/skills",
    binary: "cursor",
  },
  HarnessDef {
    name: "claude",
    config: ".claude",
    skills: ".claude/skills",
    binary: "claude",
  },
  HarnessDef {
    name: "codex",
    config: ".codex",
    skills: ".codex/skills",
    binary: "codex",
  },
  HarnessDef {
    name: "gemini",
    config: ".gemini",
    skills: ".gemini/skills",
    binary: "gemini",
  },
  HarnessDef {
    name: "antigravity",
    config: ".gemini/antigravity",
    skills: ".gemini/antigravity/skills",
    binary: "antigravity",
  },
  HarnessDef {
    name: "copilot",
    config: ".copilot",
    skills: ".copilot/skills",
    binary: "copilot",
  },
  HarnessDef {
    name: "opencode",
    config: ".config/opencode",
    skills: ".config/opencode/skills",
    binary: "opencode",
  },
  HarnessDef {
    name: "factory",
    config: ".factory",
    skills: ".factory/skills",
    binary: "droid",
  },
  HarnessDef {
    name: "hermes",
    config: ".hermes",
    skills: ".hermes/skills",
    binary: "hermes",
  },
  HarnessDef {
    name: "grok",
    config: ".grok",
    skills: ".grok/skills",
    binary: "grok",
  },
  HarnessDef {
    name: "kilo",
    config: ".kilocode",
    skills: ".kilocode/skills",
    binary: "kilo",
  },
  HarnessDef {
    name: "qwen",
    config: ".qwen",
    skills: ".qwen/skills",
    binary: "qwen",
  },
  HarnessDef {
    name: "kiro",
    config: ".kiro",
    skills: ".kiro/skills",
    binary: "kiro-cli",
  },
  HarnessDef {
    name: "pi",
    config: ".pi",
    skills: ".pi/agent/skills",
    binary: "pi",
  },
  HarnessDef {
    name: "goose",
    config: ".config/goose",
    skills: ".config/goose/skills",
    binary: "goose",
  },
  HarnessDef {
    name: "amp",
    config: ".config/amp",
    skills: ".config/amp/skills",
    binary: "amp",
  },
];

pub const EXISTING: &[&str] = &[
  "trae-cn",
  "codemaker",
  "trae",
  "codestudio",
  "roo",
  "zencoder",
  "codeartsdoer",
  "commandcode",
  "agent",
  "codebuddy",
  "zcode",
];

pub const ALIASES: &[(&str, &str)] = &[
  ("opencode-legacy", ".config/opencode/skill"),
  ("opencode-home", ".opencode/skills"),
  ("skills-home", ".skills"),
  ("kimi-brand", ".kimi/skills"),
  ("agents-config", ".config/agents/skills"),
  // T3 Code Antigravity scans this path (see shipped `discoverAntigravitySkills`).
  ("gemini-config", ".gemini/config/skills"),
];

pub const MANAGED_PARTS: &[&str] = &[
  "plugins",
  ".plugins",
  "extensions",
  "vendor",
  "bundled",
  "skills-cursor",
  "installed-plugins",
  ".system",
];
