use std::collections::HashMap;
use std::sync::RwLock;

/// Namespace access control
#[derive(Debug, Clone)]
pub struct NamespacePermission {
    pub read: bool,
    pub write: bool,
    pub delete: bool,
    pub reason: bool,
}

impl Default for NamespacePermission {
    fn default() -> Self {
        Self {
            read: true,
            write: true,
            delete: true,
            reason: true,
        }
    }
}

/// Auth layer for namespace-based access control
pub struct NamespaceAuth {
    /// Token -> (namespace patterns, permissions)
    tokens: RwLock<HashMap<String, (Vec<String>, NamespacePermission)>>,
    /// Allow unauthenticated access to "default" namespace
    pub allow_anonymous_default: bool,
}

impl Default for NamespaceAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl NamespaceAuth {
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
            allow_anonymous_default: true,
        }
    }

    /// Register a token with access to specific namespaces
    pub fn register_token(&self, token: &str, namespaces: Vec<String>, permissions: NamespacePermission) {
        let mut tokens = self.tokens.write().unwrap();
        tokens.insert(token.to_string(), (namespaces, permissions));
    }

    /// Check if token has permission for namespace and operation
    pub fn check(&self, token: Option<&str>, namespace: &str, operation: &str) -> Result<(), String> {
        // Anonymous access to default namespace
        if token.is_none() && namespace == "default" && self.allow_anonymous_default {
            return Ok(());
        }

        let token = token.ok_or("Authentication required")?;
        let tokens = self.tokens.read().unwrap();
        
        let (patterns, perms) = tokens.get(token)
            .ok_or("Invalid token")?;

        // Check namespace pattern match
        let ns_match = patterns.iter().any(|p| {
            if p == "*" { true }
            else if p.ends_with('*') { namespace.starts_with(&p[..p.len()-1]) }
            else { p == namespace }
        });

        if !ns_match {
            return Err(format!("Token not authorized for namespace: {}", namespace));
        }

        // Check operation permission
        match operation {
            "read" if !perms.read => Err("Read permission denied".to_string()),
            "write" if !perms.write => Err("Write permission denied".to_string()),
            "delete" if !perms.delete => Err("Delete permission denied".to_string()),
            "reason" if !perms.reason => Err("Reasoning permission denied".to_string()),
            _ => Ok(()),
        }
    }

    /// Load tokens from environment variable (JSON format)
    pub fn load_from_env(&self) {
        if let Ok(json) = std::env::var("SYNAPSE_AUTH_TOKENS") {
            if let Ok(map) = serde_json::from_str::<HashMap<String, Vec<String>>>(&json) {
                for (token, namespaces) in map {
                    self.register_token(&token, namespaces, NamespacePermission::default());
                }
            }
        }
    }
}
