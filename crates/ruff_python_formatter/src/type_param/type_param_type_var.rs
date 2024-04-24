use ruff_formatter::write;
use ruff_python_ast::TypeParamTypeVar;

use crate::prelude::*;

#[derive(Default)]
pub struct FormatTypeParamTypeVar;

impl FormatNodeRule<TypeParamTypeVar> for FormatTypeParamTypeVar {
    fn fmt_fields(&self, item: &TypeParamTypeVar, f: &mut PyFormatter) -> FormatResult<()> {
        let TypeParamTypeVar {
            range: _,
            name,
            bound,
            default_value,
        } = item;
        name.format().fmt(f)?;
        if let Some(bound) = bound {
            write!(f, [token(":"), space(), bound.format()])?;
        }
        if let Some(default_value) = default_value {
            write!(f, [space(), token("="), space(), default_value.format()])?;
        }
        Ok(())
    }
}
