use crate::accounts::Account;
use crate::panel_client::PanelClient;
use crate::state::AppState;

pub(crate) struct PanelContext {
    pub panel: String,
    pub token: Option<String>,
    pub selected_instance: Option<String>,
}

pub(crate) fn current(state: &AppState) -> Option<PanelContext> {
    let settings = state.settings.read().unwrap();
    let panel = AppState::panel_url_from_settings(&settings)?;
    let accounts = state.accounts.read().unwrap();
    let token = accounts.active().and_then(|account| token_for_account(state, account, &panel));
    Some(PanelContext { panel, token, selected_instance: settings.selected_instance.clone() })
}

pub(crate) fn token_for_panel(state: &AppState, panel: &str) -> Option<String> {
    let accounts = state.accounts.read().unwrap();
    token_for_account(state, accounts.active()?, panel)
}

pub(crate) fn token_for_account(state: &AppState, account: &Account, panel: &str) -> Option<String> {
    (account.kind == "panel")
        .then_some(account.panel_url.as_deref())
        .flatten()
        .filter(|account_panel| PanelClient::same_base(account_panel, panel))
        .and_then(|_| state.secrets.get(&account.id).panel_token)
}

#[cfg(test)]
mod tests;
