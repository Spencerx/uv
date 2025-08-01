use std::path::PathBuf;
use std::{collections::BTreeMap, num::NonZeroUsize};

use url::Url;

use uv_configuration::{
    ConfigSettings, ExportFormat, IndexStrategy, KeyringProviderType, PackageConfigSettings,
    RequiredVersion, TargetTriple, TrustedPublishing,
};
use uv_distribution_types::{Index, IndexUrl, PipExtraIndex, PipFindLinks, PipIndex};
use uv_install_wheel::LinkMode;
use uv_pypi_types::{SchemaConflicts, SupportedEnvironments};
use uv_python::{PythonDownloads, PythonPreference, PythonVersion};
use uv_redacted::DisplaySafeUrl;
use uv_resolver::{
    AnnotationStyle, ExcludeNewer, ExcludeNewerPackage, ExcludeNewerTimestamp, ForkStrategy,
    PrereleaseMode, ResolutionMode,
};
use uv_torch::TorchMode;
use uv_workspace::pyproject::ExtraBuildDependencies;
use uv_workspace::pyproject_mut::AddBoundsKind;

use crate::{FilesystemOptions, Options, PipOptions};

pub trait Combine {
    /// Combine two values, preferring the values in `self`.
    ///
    /// The logic should follow that of Cargo's `config.toml`:
    ///
    /// > If a key is specified in multiple config files, the values will get merged together.
    /// > Numbers, strings, and booleans will use the value in the deeper config directory taking
    /// > precedence over ancestor directories, where the home directory is the lowest priority.
    /// > Arrays will be joined together with higher precedence items being placed later in the
    /// > merged array.
    ///
    /// ...with one exception: we place items with higher precedence earlier in the merged array.
    #[must_use]
    fn combine(self, other: Self) -> Self;
}

impl Combine for Option<FilesystemOptions> {
    /// Combine the options used in two [`FilesystemOptions`]s. Retains the root of `self`.
    fn combine(self, other: Option<FilesystemOptions>) -> Option<FilesystemOptions> {
        match (self, other) {
            (Some(a), Some(b)) => Some(FilesystemOptions(
                a.into_options().combine(b.into_options()),
            )),
            (a, b) => a.or(b),
        }
    }
}

impl Combine for Option<Options> {
    /// Combine the options used in two [`Options`]s. Retains the root of `self`.
    fn combine(self, other: Option<Options>) -> Option<Options> {
        match (self, other) {
            (Some(a), Some(b)) => Some(a.combine(b)),
            (a, b) => a.or(b),
        }
    }
}

impl Combine for Option<PipOptions> {
    fn combine(self, other: Option<PipOptions>) -> Option<PipOptions> {
        match (self, other) {
            (Some(a), Some(b)) => Some(a.combine(b)),
            (a, b) => a.or(b),
        }
    }
}

macro_rules! impl_combine_or {
    ($name:ident) => {
        impl Combine for Option<$name> {
            fn combine(self, other: Option<$name>) -> Option<$name> {
                self.or(other)
            }
        }
    };
}

impl_combine_or!(AddBoundsKind);
impl_combine_or!(AnnotationStyle);
impl_combine_or!(ExcludeNewer);
impl_combine_or!(ExcludeNewerTimestamp);
impl_combine_or!(ExportFormat);
impl_combine_or!(ForkStrategy);
impl_combine_or!(Index);
impl_combine_or!(IndexStrategy);
impl_combine_or!(IndexUrl);
impl_combine_or!(KeyringProviderType);
impl_combine_or!(LinkMode);
impl_combine_or!(DisplaySafeUrl);
impl_combine_or!(NonZeroUsize);
impl_combine_or!(PathBuf);
impl_combine_or!(PipExtraIndex);
impl_combine_or!(PipFindLinks);
impl_combine_or!(PipIndex);
impl_combine_or!(PrereleaseMode);
impl_combine_or!(PythonDownloads);
impl_combine_or!(PythonPreference);
impl_combine_or!(PythonVersion);
impl_combine_or!(RequiredVersion);
impl_combine_or!(ResolutionMode);
impl_combine_or!(SchemaConflicts);
impl_combine_or!(String);
impl_combine_or!(SupportedEnvironments);
impl_combine_or!(TargetTriple);
impl_combine_or!(TorchMode);
impl_combine_or!(TrustedPublishing);
impl_combine_or!(Url);
impl_combine_or!(bool);

impl<T> Combine for Option<Vec<T>> {
    /// Combine two vectors by extending the vector in `self` with the vector in `other`, if they're
    /// both `Some`.
    fn combine(self, other: Option<Vec<T>>) -> Option<Vec<T>> {
        match (self, other) {
            (Some(mut a), Some(b)) => {
                a.extend(b);
                Some(a)
            }
            (a, b) => a.or(b),
        }
    }
}

impl<K: Ord, T> Combine for Option<BTreeMap<K, Vec<T>>> {
    /// Combine two maps of vecs by combining their vecs
    fn combine(self, other: Option<BTreeMap<K, Vec<T>>>) -> Option<BTreeMap<K, Vec<T>>> {
        match (self, other) {
            (Some(mut a), Some(b)) => {
                for (key, value) in b {
                    a.entry(key).or_default().extend(value);
                }
                Some(a)
            }
            (a, b) => a.or(b),
        }
    }
}

impl Combine for Option<ExcludeNewerPackage> {
    /// Combine two [`ExcludeNewerPackage`] instances by merging them, with the values in `self` taking precedence.
    fn combine(self, other: Option<ExcludeNewerPackage>) -> Option<ExcludeNewerPackage> {
        match (self, other) {
            (Some(mut a), Some(b)) => {
                // Extend with values from b, but a takes precedence (we don't overwrite existing keys)
                for (key, value) in b {
                    a.entry(key).or_insert(value);
                }
                Some(a)
            }
            (a, b) => a.or(b),
        }
    }
}

impl Combine for Option<ConfigSettings> {
    /// Combine two maps by merging the map in `self` with the map in `other`, if they're both
    /// `Some`.
    fn combine(self, other: Option<ConfigSettings>) -> Option<ConfigSettings> {
        match (self, other) {
            (Some(a), Some(b)) => Some(a.merge(b)),
            (a, b) => a.or(b),
        }
    }
}

impl Combine for Option<PackageConfigSettings> {
    /// Combine two maps by merging the map in `self` with the map in `other`, if they're both
    /// `Some`.
    fn combine(self, other: Option<PackageConfigSettings>) -> Option<PackageConfigSettings> {
        match (self, other) {
            (Some(a), Some(b)) => Some(a.merge(b)),
            (a, b) => a.or(b),
        }
    }
}

impl Combine for serde::de::IgnoredAny {
    fn combine(self, _other: Self) -> Self {
        self
    }
}

impl Combine for Option<serde::de::IgnoredAny> {
    fn combine(self, _other: Self) -> Self {
        self
    }
}

impl Combine for ExcludeNewer {
    fn combine(mut self, other: Self) -> Self {
        self.global = self.global.combine(other.global);

        if !other.package.is_empty() {
            if self.package.is_empty() {
                self.package = other.package;
            } else {
                // Merge package-specific timestamps, with self taking precedence
                for (pkg, timestamp) in &other.package {
                    self.package.entry(pkg.clone()).or_insert(*timestamp);
                }
            }
        }

        self
    }
}

impl Combine for ExtraBuildDependencies {
    fn combine(mut self, other: Self) -> Self {
        for (key, value) in other {
            match self.entry(key) {
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    // Combine the vecs, with self taking precedence
                    let existing = entry.get_mut();
                    existing.extend(value);
                }
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(value);
                }
            }
        }
        self
    }
}

impl Combine for Option<ExtraBuildDependencies> {
    fn combine(self, other: Option<ExtraBuildDependencies>) -> Option<ExtraBuildDependencies> {
        match (self, other) {
            (Some(a), Some(b)) => Some(a.combine(b)),
            (a, b) => a.or(b),
        }
    }
}
