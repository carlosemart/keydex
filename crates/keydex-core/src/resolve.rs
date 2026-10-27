//! Interpolation resolvers.
//!
//! A resolver is a pure function invoked by `${name:arg,arg}`. The engine ships `env` and
//! a caller can register more without touching the core.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::value::Value;

/// A `${name:args}` resolver.
pub trait Resolver: Send + Sync {
    /// Name used in the placeholder.
    fn name(&self) -> &str;

    /// Resolves the call. The error is a user-facing message.
    ///
    /// # Errors
    ///
    /// Returns a message when the arguments are invalid or the value cannot be produced.
    fn resolve(
        &self,
        args: &[&str],
        env: &BTreeMap<String, String>,
    ) -> std::result::Result<Value, String>;
}

/// Registry of resolvers available to interpolation.
#[derive(Clone, Default)]
pub struct Resolvers {
    map: BTreeMap<String, Arc<dyn Resolver>>,
}

impl std::fmt::Debug for Resolvers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resolvers")
            .field("names", &self.map.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Resolvers {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registry with the built-in resolvers (`env`).
    #[must_use]
    pub fn with_builtins() -> Self {
        let mut resolvers = Self::default();
        resolvers.register(Arc::new(EnvResolver));
        resolvers
    }

    /// Registers a resolver, replacing one with the same name.
    pub fn register(&mut self, resolver: Arc<dyn Resolver>) {
        self.map.insert(resolver.name().to_string(), resolver);
    }

    /// Looks up a resolver by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Resolver>> {
        self.map.get(name)
    }
}

/// `${env:VAR}`: reads a captured environment variable.
#[derive(Debug, Default, Clone, Copy)]
pub struct EnvResolver;

impl Resolver for EnvResolver {
    fn name(&self) -> &'static str {
        "env"
    }

    fn resolve(
        &self,
        args: &[&str],
        env: &BTreeMap<String, String>,
    ) -> std::result::Result<Value, String> {
        match args {
            [name] => env
                .get(*name)
                .map(|value| Value::str(value.as_str()))
                .ok_or_else(|| format!("environment variable {name} is not set")),
            _ => Err(String::from("env expects exactly one argument")),
        }
    }
}
