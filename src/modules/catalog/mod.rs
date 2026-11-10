//! Describe bundled modules independently of their presentation and runtime.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModuleId {
    Git,
    Pipelines,
    Terminal,
    Deployment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    System,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Module {
    pub id: ModuleId,
    pub origin: Origin,
}

pub(crate) const MODULES: &[Module] = &[
    Module {
        id: ModuleId::Git,
        origin: Origin::System,
    },
    Module {
        id: ModuleId::Pipelines,
        origin: Origin::System,
    },
    Module {
        id: ModuleId::Terminal,
        origin: Origin::System,
    },
    Module {
        id: ModuleId::Deployment,
        origin: Origin::System,
    },
];

impl ModuleId {
    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Pipelines => "pipelines",
            Self::Terminal => "terminal",
            Self::Deployment => "deployment",
        }
    }
}
