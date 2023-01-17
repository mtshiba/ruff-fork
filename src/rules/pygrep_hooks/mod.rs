//! Rules from [pygrep-hooks](https://github.com/pre-commit/pygrep-hooks).
pub(crate) mod rules;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;
    use test_case::test_case;

    use crate::linter::test_path;
    use crate::registry::RuleCode;
    use crate::settings;

    #[test_case(RuleCode::PGH001, Path::new("PGH001_0.py"); "PGH001_0")]
    #[test_case(RuleCode::PGH001, Path::new("PGH001_1.py"); "PGH001_1")]
    #[test_case(RuleCode::PGH002, Path::new("PGH002_0.py"); "PGH002_0")]
    #[test_case(RuleCode::PGH002, Path::new("PGH002_1.py"); "PGH002_1")]
    #[test_case(RuleCode::PGH003, Path::new("PGH003_0.py"); "PGH003_0")]
    #[test_case(RuleCode::PGH004, Path::new("PGH004_0.py"); "PGH004_0")]
    fn rules(rule_code: RuleCode, path: &Path) -> Result<()> {
        let snapshot = format!("{}_{}", rule_code.code(), path.to_string_lossy());
        let diagnostics = test_path(
            Path::new("./resources/test/fixtures/pygrep-hooks")
                .join(path)
                .as_path(),
            &settings::Settings::for_rule(rule_code),
        )?;
        insta::assert_yaml_snapshot!(snapshot, diagnostics);
        Ok(())
    }
}
