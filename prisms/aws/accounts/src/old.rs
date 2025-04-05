use blue_core::prelude::*;
use serde_json::json;
use std::path::PathBuf;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct FavoriteAccount {
    #[serde(rename = "accountId")]
    account_id: String,
    #[serde(rename = "accountName")]
    account_name: String,
    policy: String,
    #[serde(rename = "sessionDuration")]
    session_duration: String,
    partition: String,
}

#[derive(Debug, Deserialize)]
struct FavoritesResponse {
    #[serde(rename = "favoriteAccountList")]
    favorite_account_list: Vec<FavoriteAccount>,
}

pub struct AccountsModule {
    manifest: ModuleManifest,
    context: ModuleContext,
}

impl AccountsModule {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| Error::Module(format!("Failed to get home directory: {}", e)))?;

        let lib_path = home.join(".blue/modules");
        let manifest = ModuleManifest::load(lib_path.join("aws").join("accounts").join("manifest.toml"))?;
        let context = ModuleContext::new(lib_path);

        Ok(Self { manifest, context })
    }

    fn build_console_url(account: &FavoriteAccount) -> String {
        format!(
            "https://iad.merlon.amazon.dev/console?awsAccountId={}&awsPartition={}&accountName={}&sessionDuration={}&policy=arn:aws:iam::aws:policy/{}",
            account.account_id,
            account.partition,
            account.account_name,
            account.session_duration,
            account.policy
        )
    }

    fn handle_list(&mut self) -> Result<Value> {
        // Call curl module to get favorites
        let response = self.context.call_module("aws:curl", &["get"], json!({
            "url": "https://conduit.security.a2z.com/api/accounts/favorites"
        }), None, None)?;

        let body = response.get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Module("Invalid response format from curl module".into()))?;

        // Parse response
        let favorites: FavoritesResponse = serde_json::from_str(body)
            .map_err(|e| Error::Module(format!("Failed to parse response: {}", e)))?;

        // Format accounts for display
        let accounts = favorites.favorite_account_list.iter()
            .map(|account| {
                json!({
                    "accountId": account.account_id,
                    "accountName": account.account_name,
                    "displayUrl": format!("🔗 Console ({})", account.account_name),
                    "consoleUrl": Self::build_console_url(account)
                })
            })
            .collect::<Vec<_>>();

        Ok(json!({
            "accounts": accounts
        }))
    }
}

impl Module for AccountsModule {
    fn name(&self) -> &str {
        &self.manifest.module.name
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn call(&mut self, path: &[&str], _args: Value, _stdout: Option<&mut dyn Write>, _stderr: Option<&mut dyn Write>) -> Result<Value> {
        // Validate method exists
        if self.manifest.find_method(path).is_none() {
            return Err(Error::MethodNotFound(path.join(" ")));
        }

        match path {
            ["list"] => self.handle_list(),
            _ => Err(Error::MethodNotFound(path.join(" ")))
        }
    }
}

#[no_mangle]
pub fn create_module() -> Box<dyn Module> {
    Box::new(AccountsModule::new().expect("Failed to create accounts module"))
}
