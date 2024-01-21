use std::path::{Path, PathBuf};

use anyhow::Result;
use log::debug;
use path_absolutize::path_dedot;

use ruff_workspace::configuration::Configuration;
use ruff_workspace::pyproject;
use ruff_workspace::resolver::{
    resolve_root_settings, ConfigurationTransformer, PyprojectConfig, PyprojectDiscoveryStrategy,
    Relativity,
};

use crate::args::ConfigArgs;

/// Resolve the relevant settings strategy and defaults for the current
/// invocation.
pub fn resolve(
    isolated: bool,
    config_args: &ConfigArgs,
    stdin_filename: Option<&Path>,
) -> Result<PyprojectConfig> {
    // First priority: if we're running in isolated mode, use the default settings.
    if isolated {
        let config = config_args.transform(Configuration::default());
        let settings = config.into_settings(&path_dedot::CWD)?;
        debug!("Isolated mode, not reading any pyproject.toml");
        return Ok(PyprojectConfig::new(
            PyprojectDiscoveryStrategy::Fixed,
            settings,
            None,
        ));
    }

    // Second priority: the user specified a `pyproject.toml` file. Use that
    // `pyproject.toml` for _all_ configuration, and resolve paths relative to the
    // current working directory. (This matches ESLint's behavior.)
    if let Some(pyproject) = config_args
        .config_file()
        .map(|config| config.display().to_string())
        .map(|config| shellexpand::full(&config).map(|config| PathBuf::from(config.as_ref())))
        .transpose()?
    {
        let settings = resolve_root_settings(&pyproject, Relativity::Cwd, config_args)?;
        debug!(
            "Using user-specified configuration file at: {}",
            pyproject.display()
        );
        return Ok(PyprojectConfig::new(
            PyprojectDiscoveryStrategy::Fixed,
            settings,
            Some(pyproject),
        ));
    }

    // Third priority: find a `pyproject.toml` file in either an ancestor of
    // `stdin_filename` (if set) or the current working path all paths relative to
    // that directory. (With `Strategy::Hierarchical`, we'll end up finding
    // the "closest" `pyproject.toml` file for every Python file later on,
    // so these act as the "default" settings.)
    if let Some(pyproject) = pyproject::find_settings_toml(
        stdin_filename
            .as_ref()
            .unwrap_or(&path_dedot::CWD.as_path()),
    )? {
        debug!(
            "Using configuration file (via parent) at: {}",
            pyproject.display()
        );
        let settings = resolve_root_settings(&pyproject, Relativity::Parent, config_args)?;
        return Ok(PyprojectConfig::new(
            PyprojectDiscoveryStrategy::Hierarchical,
            settings,
            Some(pyproject),
        ));
    }

    // Fourth priority: find a user-specific `pyproject.toml`, but resolve all paths
    // relative the current working directory. (With `Strategy::Hierarchical`, we'll
    // end up the "closest" `pyproject.toml` file for every Python file later on, so
    // these act as the "default" settings.)
    if let Some(pyproject) = pyproject::find_user_settings_toml() {
        debug!(
            "Using configuration file (via cwd) at: {}",
            pyproject.display()
        );
        let settings = resolve_root_settings(&pyproject, Relativity::Cwd, config_args)?;
        return Ok(PyprojectConfig::new(
            PyprojectDiscoveryStrategy::Hierarchical,
            settings,
            Some(pyproject),
        ));
    }

    // Fallback: load Ruff's default settings, and resolve all paths relative to the
    // current working directory. (With `Strategy::Hierarchical`, we'll end up the
    // "closest" `pyproject.toml` file for every Python file later on, so these act
    // as the "default" settings.)
    debug!("Using Ruff default settings");
    let config = config_args.transform(Configuration::default());
    let settings = config.into_settings(&path_dedot::CWD)?;
    Ok(PyprojectConfig::new(
        PyprojectDiscoveryStrategy::Hierarchical,
        settings,
        None,
    ))
}
