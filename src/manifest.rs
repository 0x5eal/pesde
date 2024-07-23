use crate::{
    names::{PackageName, PackageNames},
    source::{DependencySpecifiers, VersionId},
};
use relative_path::RelativePathBuf;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Display, Formatter},
    str::FromStr,
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TargetKind {
    #[cfg(feature = "roblox")]
    Roblox,
    #[cfg(feature = "lune")]
    Lune,
    #[cfg(feature = "luau")]
    Luau,
}

impl Display for TargetKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "roblox")]
            TargetKind::Roblox => write!(f, "roblox"),
            #[cfg(feature = "lune")]
            TargetKind::Lune => write!(f, "lune"),
            #[cfg(feature = "luau")]
            TargetKind::Luau => write!(f, "luau"),
        }
    }
}

impl FromStr for TargetKind {
    type Err = errors::TargetKindFromStr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(feature = "roblox")]
            "roblox" => Ok(Self::Roblox),
            #[cfg(feature = "lune")]
            "lune" => Ok(Self::Lune),
            #[cfg(feature = "luau")]
            "luau" => Ok(Self::Luau),
            t => Err(errors::TargetKindFromStr::Unknown(t.to_string())),
        }
    }
}

impl TargetKind {
    pub const VARIANTS: &'static [TargetKind] = &[
        #[cfg(feature = "roblox")]
        TargetKind::Roblox,
        #[cfg(feature = "lune")]
        TargetKind::Lune,
        #[cfg(feature = "luau")]
        TargetKind::Luau,
    ];

    // self is the project's target, dependency is the target of the dependency
    pub fn is_compatible_with(&self, dependency: &Self) -> bool {
        if self == dependency {
            return true;
        }

        match (self, dependency) {
            #[cfg(all(feature = "lune", feature = "luau"))]
            (TargetKind::Lune, TargetKind::Luau) => true,

            _ => false,
        }
    }

    pub fn packages_folder(&self, dependency: &Self) -> String {
        if self == dependency {
            return "packages".to_string();
        }

        format!("{dependency}_packages")
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case", tag = "environment")]
pub enum Target {
    #[cfg(feature = "roblox")]
    Roblox {
        #[serde(default)]
        lib: Option<RelativePathBuf>,
        #[serde(default)]
        build_files: BTreeSet<String>,
    },
    #[cfg(feature = "lune")]
    Lune {
        #[serde(default)]
        lib: Option<RelativePathBuf>,
        #[serde(default)]
        bin: Option<RelativePathBuf>,
    },
    #[cfg(feature = "luau")]
    Luau {
        #[serde(default)]
        lib: Option<RelativePathBuf>,
        #[serde(default)]
        bin: Option<RelativePathBuf>,
    },
}

impl Target {
    pub fn kind(&self) -> TargetKind {
        match self {
            #[cfg(feature = "roblox")]
            Target::Roblox { .. } => TargetKind::Roblox,
            #[cfg(feature = "lune")]
            Target::Lune { .. } => TargetKind::Lune,
            #[cfg(feature = "luau")]
            Target::Luau { .. } => TargetKind::Luau,
        }
    }

    pub fn lib_path(&self) -> Option<&RelativePathBuf> {
        match self {
            #[cfg(feature = "roblox")]
            Target::Roblox { lib, .. } => lib.as_ref(),
            #[cfg(feature = "lune")]
            Target::Lune { lib, .. } => lib.as_ref(),
            #[cfg(feature = "luau")]
            Target::Luau { lib, .. } => lib.as_ref(),
        }
    }

    pub fn bin_path(&self) -> Option<&RelativePathBuf> {
        match self {
            #[cfg(feature = "roblox")]
            Target::Roblox { .. } => None,
            #[cfg(feature = "lune")]
            Target::Lune { bin, .. } => bin.as_ref(),
            #[cfg(feature = "luau")]
            Target::Luau { bin, .. } => bin.as_ref(),
        }
    }

    pub fn validate_publish(&self) -> Result<(), errors::TargetValidatePublishError> {
        let has_exports = match self {
            #[cfg(feature = "roblox")]
            Target::Roblox { lib, .. } => lib.is_some(),
            #[cfg(feature = "lune")]
            Target::Lune { lib, bin } => lib.is_some() || bin.is_some(),
            #[cfg(feature = "luau")]
            Target::Luau { lib, bin } => lib.is_some() || bin.is_some(),
        };

        if !has_exports {
            return Err(errors::TargetValidatePublishError::NoExportedFiles);
        }

        match self {
            #[cfg(feature = "roblox")]
            Target::Roblox { build_files, .. } if build_files.is_empty() => {
                Err(errors::TargetValidatePublishError::NoBuildFiles)
            }

            _ => Ok(()),
        }
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind())
    }
}

#[derive(
    Debug, DeserializeFromStr, SerializeDisplay, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct OverrideKey(pub Vec<Vec<String>>);

impl FromStr for OverrideKey {
    type Err = errors::OverrideKeyFromStr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let overrides = s
            .split(',')
            .map(|overrides| overrides.split('>').map(|s| s.to_string()).collect())
            .collect::<Vec<Vec<String>>>();

        if overrides.is_empty() {
            return Err(errors::OverrideKeyFromStr::Empty);
        }

        Ok(Self(overrides))
    }
}

impl Display for OverrideKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|overrides| {
                    overrides
                        .iter()
                        .map(|o| o.as_str())
                        .collect::<Vec<_>>()
                        .join(">")
                })
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScriptName {
    #[cfg(feature = "roblox")]
    RobloxSyncConfigGenerator,
    #[cfg(feature = "wally-compat")]
    SourcemapGenerator,
}

impl Display for ScriptName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "roblox")]
            ScriptName::RobloxSyncConfigGenerator => write!(f, "roblox_sync_config_generator"),
            #[cfg(feature = "wally-compat")]
            ScriptName::SourcemapGenerator => write!(f, "sourcemap_generator"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub name: PackageName,
    pub version: Version,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    pub target: Target,
    #[serde(default)]
    pub private: bool,
    #[serde(default, skip_serializing)]
    pub scripts: BTreeMap<String, RelativePathBuf>,
    #[serde(default)]
    pub indices: BTreeMap<String, url::Url>,
    #[cfg(feature = "wally-compat")]
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub wally_indices: BTreeMap<String, url::Url>,
    #[serde(default, skip_serializing)]
    pub overrides: BTreeMap<OverrideKey, DependencySpecifiers>,
    #[serde(default)]
    pub includes: BTreeSet<String>,
    #[cfg(feature = "patches")]
    #[serde(default, skip_serializing)]
    pub patches: BTreeMap<PackageNames, BTreeMap<VersionId, RelativePathBuf>>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, DependencySpecifiers>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub peer_dependencies: BTreeMap<String, DependencySpecifiers>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dev_dependencies: BTreeMap<String, DependencySpecifiers>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Standard,
    Dev,
    Peer,
}

impl Manifest {
    pub fn all_dependencies(
        &self,
    ) -> Result<
        BTreeMap<String, (DependencySpecifiers, DependencyType)>,
        errors::AllDependenciesError,
    > {
        let mut all_deps = BTreeMap::new();

        for (deps, ty) in [
            (&self.dependencies, DependencyType::Standard),
            (&self.peer_dependencies, DependencyType::Peer),
            (&self.dev_dependencies, DependencyType::Dev),
        ] {
            for (alias, spec) in deps {
                if all_deps.insert(alias.clone(), (spec.clone(), ty)).is_some() {
                    return Err(errors::AllDependenciesError::AliasConflict(alias.clone()));
                }
            }
        }

        Ok(all_deps)
    }
}

pub mod errors {
    use thiserror::Error;

    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum OverrideKeyFromStr {
        #[error("empty override key")]
        Empty,
    }

    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum AllDependenciesError {
        #[error("another specifier is already using the alias {0}")]
        AliasConflict(String),
    }

    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum TargetValidatePublishError {
        #[error("no exported files specified")]
        NoExportedFiles,

        #[cfg(feature = "roblox")]
        #[error("roblox target must have at least one build file")]
        NoBuildFiles,
    }

    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum TargetKindFromStr {
        #[error("unknown target kind {0}")]
        Unknown(String),
    }
}
