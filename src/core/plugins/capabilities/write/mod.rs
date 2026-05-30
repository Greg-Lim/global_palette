use wasmtime::Linker;

use super::PluginStoreState;

mod text;

pub(crate) fn register(linker: &mut Linker<PluginStoreState>) -> Result<(), String> {
    text::register(linker)?;
    Ok(())
}
