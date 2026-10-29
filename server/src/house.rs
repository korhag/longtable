// ------------------------------------------------------------
// Module: house
// Purpose: Accounts, friendships, and the list of tables on
//          this machine. Passwords are stored hashed. The
//          live game still sits in one open table.
// ------------------------------------------------------------

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const COLORS: [&str; 6] = ["#d4a656", "#7eb8a2", "#d27b66", "#8aa6d4", "#c98bb8", "#d8c7a1"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountSettings {
    pub theme: String,
    pub parchment: String,
    pub gold: String,
    pub dice_tray: String,
    pub dice_corner: String,
}

impl Default for AccountSettings {
    fn default() -> Self {
        Self {
            theme: "hearth".into(),
            parchment: "#f4ecdf".into(),
            gold: "#d4a656".into(),
            dice_tray: "quarter".into(),
            dice_corner: "bottom-right".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Account {
    id: String,
    username: String,
    password: String,
    color: String,
    #[serde(default)]
    settings: AccountSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Session {
    token: String,
    account_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TableCard {
    id: String,
    name: String,
    owner_id: String,
    #[serde(default)]
    owner_seat: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Membership {
    table_id: String,
    account_id: String,
    standing: String,
    #[serde(default)]
    seat_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Friendship {
    from_id: String,
    to_id: String,
    accepted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct HouseFile {
    #[serde(default)]
    accounts: Vec<Account>,
    #[serde(default)]
    sessions: Vec<Session>,
    #[serde(default)]
    tables: Vec<TableCard>,
    #[serde(default)]
    memberships: Vec<Membership>,
    #[serde(default)]
    friends: Vec<Friendship>,
    #[serde(default)]
    open_id: Option<String>,
    #[serde(default)]
    next_color: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicAccount {
    pub id: String,
    pub username: String,
    pub color: String,
    pub settings: AccountSettings,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    pub id: String,
    pub name: String,
    pub yours: bool,
    pub standing: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonRow {
    pub id: String,
    pub username: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeatRequestRow {
    pub table_id: String,
    pub table_name: String,
    pub account_id: String,
    pub username: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTableRow {
    pub id: String,
    pub name: String,
    pub owner_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lobby {
    pub account: PublicAccount,
    pub tables: Vec<TableRow>,
    pub requests: Vec<SeatRequestRow>,
    pub friends: Vec<PersonRow>,
    pub incoming: Vec<PersonRow>,
    pub friend_tables: Vec<OpenTableRow>,
    pub open_id: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Door {
    Owner { seat: Option<String> },
    Always { seat: Option<String> },
    Once { seat: Option<String> },
    Spent { seat: Option<String> },
    Closed,
}

pub struct House {
    path: PathBuf,
    file: HouseFile,
}

impl House {
    /// Load the house. On the first boot, make a host account from the
    /// table passphrase and claim the campaign already on disk.
    pub fn load(data_dir: &Path, passphrase: &str, campaign_name: &str) -> (Self, bool) {
        let path = data_dir.join("house.json");
        if path.is_file() {
            let text = fs::read_to_string(&path).unwrap_or_default();
            let file = serde_json::from_str(&text).unwrap_or_default();
            return (Self { path, file }, false);
        }
        let mut house = Self { path, file: HouseFile::default() };
        let account = house.insert_account("host", passphrase);
        let card = TableCard {
            id: Uuid::new_v4().to_string(),
            name: clean_label(campaign_name, "Table"),
            owner_id: account.id,
            owner_seat: None,
        };
        house.file.open_id = Some(card.id.clone());
        house.file.tables.push(card);
        house.save();
        (house, true)
    }

    pub fn open_id(&self) -> Option<String> {
        self.file.open_id.clone()
    }

    pub fn set_open_id(&mut self, id: Option<String>) {
        self.file.open_id = id;
        self.save();
    }

    pub fn table_name(&self, id: &str) -> Option<String> {
        self.file.tables.iter().find(|card| card.id == id).map(|card| card.name.clone())
    }

    pub fn owns(&self, account_id: &str, table_id: &str) -> bool {
        self.file.tables.iter().any(|card| card.id == table_id && card.owner_id == account_id)
    }

    pub fn register(&mut self, username: &str, password: &str) -> Result<(String, PublicAccount), String> {
        let username = clean_username(username)?;
        check_password(password)?;
        if self.file.accounts.iter().any(|account| account.username.eq_ignore_ascii_case(&username)) {
            return Err("that name is already taken".into());
        }
        let account = self.insert_account(&username, password);
        let token = self.issue(&account.id);
        let public = public_account(&account);
        self.save();
        Ok((token, public))
    }

    pub fn login(&mut self, username: &str, password: &str) -> Result<(String, PublicAccount), String> {
        let username = username.trim();
        let account = self
            .file
            .accounts
            .iter()
            .find(|account| account.username.eq_ignore_ascii_case(username))
            .cloned()
            .ok_or_else(|| "that name or password does not match".to_string())?;
        if !password_matches(&account.password, password) {
            return Err("that name or password does not match".into());
        }
        let token = self.issue(&account.id);
        let public = public_account(&account);
        self.save();
        Ok((token, public))
    }

    pub fn account(&self, token: &str) -> Option<PublicAccount> {
        let id = self.file.sessions.iter().find(|session| session.token == token)?.account_id.clone();
        self.file.accounts.iter().find(|account| account.id == id).map(public_account)
    }

    pub fn update_account(
        &mut self,
        token: &str,
        username: Option<String>,
        color: Option<String>,
        settings: Option<AccountSettings>,
    ) -> Result<PublicAccount, String> {
        let id = self.account_id(token)?;
        if let Some(username) = username {
            let username = clean_username(&username)?;
            let taken = self
                .file
                .accounts
                .iter()
                .any(|account| account.id != id && account.username.eq_ignore_ascii_case(&username));
            if taken {
                return Err("that name is already taken".into());
            }
            if let Some(account) = self.file.accounts.iter_mut().find(|account| account.id == id) {
                account.username = username;
            }
        }
        if let Some(color) = color {
            let color = clean_color(&color)?;
            if let Some(account) = self.file.accounts.iter_mut().find(|account| account.id == id) {
                account.color = color;
            }
        }
        if let Some(settings) = settings {
            let settings = clean_settings(settings);
            if let Some(account) = self.file.accounts.iter_mut().find(|account| account.id == id) {
                account.settings = settings;
            }
        }
        let account = self.file.accounts.iter().find(|account| account.id == id).map(public_account).expect("account");
        self.save();
        Ok(account)
    }

    pub fn create_table(&mut self, token: &str, name: &str) -> Result<String, String> {
        let owner_id = self.account_id(token)?;
        let owned = self.file.tables.iter().filter(|card| card.owner_id == owner_id).count();
        if owned >= 24 {
            return Err("you already have enough tables".into());
        }
        let id = Uuid::new_v4().to_string();
        self.file.tables.push(TableCard {
            id: id.clone(),
            name: clean_label(name, "Table"),
            owner_id,
            owner_seat: None,
        });
        self.save();
        Ok(id)
    }

    pub fn request_seat(&mut self, token: &str, table_id: &str) -> Result<(), String> {
        let account_id = self.account_id(token)?;
        if !self.file.tables.iter().any(|card| card.id == table_id) {
            return Err("that table is gone".into());
        }
        if self.owns(&account_id, table_id) {
            return Err("this is already your table".into());
        }
        if self.file.open_id.as_deref() != Some(table_id) {
            return Err("that table is not open".into());
        }
        if let Some(row) = self.file.memberships.iter_mut().find(|row| row.table_id == table_id && row.account_id == account_id) {
            if row.standing == "always" {
                return Err("you can already sit at this table".into());
            }
            row.standing = "pending".into();
            self.save();
            return Ok(());
        }
        self.file.memberships.push(Membership {
            table_id: table_id.to_string(),
            account_id,
            standing: "pending".into(),
            seat_id: None,
        });
        self.save();
        Ok(())
    }

    pub fn approve(&mut self, token: &str, table_id: &str, account_id: &str, always: bool) -> Result<(), String> {
        let owner_id = self.account_id(token)?;
        if !self.owns(&owner_id, table_id) {
            return Err("only the GM of that table can approve a seat".into());
        }
        let row = self
            .file
            .memberships
            .iter_mut()
            .find(|row| row.table_id == table_id && row.account_id == account_id)
            .ok_or_else(|| "nobody by that name is waiting".to_string())?;
        row.standing = if always { "always".into() } else { "once".into() };
        self.save();
        Ok(())
    }

    pub fn friend_request(&mut self, token: &str, username: &str) -> Result<(), String> {
        let from_id = self.account_id(token)?;
        let username = username.trim();
        let to = self
            .file
            .accounts
            .iter()
            .find(|account| account.username.eq_ignore_ascii_case(username))
            .cloned()
            .ok_or_else(|| "nobody uses that name".to_string())?;
        if to.id == from_id {
            return Err("you are already here".into());
        }
        if self.friends_with(&from_id, &to.id) {
            return Err("you are already friends".into());
        }
        if self.file.friends.iter().any(|bond| bond.from_id == from_id && bond.to_id == to.id && !bond.accepted) {
            return Ok(());
        }
        self.file.friends.push(Friendship { from_id, to_id: to.id, accepted: false });
        self.save();
        Ok(())
    }

    pub fn accept_friend(&mut self, token: &str, other_id: &str) -> Result<(), String> {
        let account_id = self.account_id(token)?;
        let bond = self
            .file
            .friends
            .iter_mut()
            .find(|bond| !bond.accepted && bond.to_id == account_id && bond.from_id == other_id)
            .ok_or_else(|| "that request is gone".to_string())?;
        bond.accepted = true;
        self.save();
        Ok(())
    }

    pub fn door(&self, account_id: &str, table_id: &str) -> Door {
        if let Some(card) = self.file.tables.iter().find(|card| card.id == table_id && card.owner_id == account_id) {
            return Door::Owner { seat: card.owner_seat.clone() };
        }
        let Some(row) = self.file.memberships.iter().find(|row| row.table_id == table_id && row.account_id == account_id) else {
            return Door::Closed;
        };
        match row.standing.as_str() {
            "always" => Door::Always { seat: row.seat_id.clone() },
            "once" => Door::Once { seat: row.seat_id.clone() },
            "spent" => Door::Spent { seat: row.seat_id.clone() },
            _ => Door::Closed,
        }
    }

    pub fn remember_seat(&mut self, table_id: &str, account_id: &str, seat_id: &str, consume_once: bool) {
        if let Some(card) = self.file.tables.iter_mut().find(|card| card.id == table_id && card.owner_id == account_id) {
            card.owner_seat = Some(seat_id.to_string());
            self.save();
            return;
        }
        if let Some(row) = self.file.memberships.iter_mut().find(|row| row.table_id == table_id && row.account_id == account_id) {
            row.seat_id = Some(seat_id.to_string());
            if consume_once && row.standing == "once" {
                row.standing = "spent".into();
            }
        }
        self.save();
    }

    pub fn lobby(&self, token: &str) -> Result<Lobby, String> {
        let account_id = self.account_id(token)?;
        let account = self.file.accounts.iter().find(|account| account.id == account_id).map(public_account).expect("account");
        let tables = self
            .file
            .tables
            .iter()
            .filter(|card| card.owner_id == account_id || self.member_of(&account_id, &card.id))
            .map(|card| TableRow {
                id: card.id.clone(),
                name: card.name.clone(),
                yours: card.owner_id == account_id,
                standing: if card.owner_id == account_id {
                    "owner".into()
                } else {
                    self.file
                        .memberships
                        .iter()
                        .find(|row| row.table_id == card.id && row.account_id == account_id)
                        .map(|row| row.standing.clone())
                        .unwrap_or_else(|| "closed".into())
                },
            })
            .collect();
        let requests = self
            .file
            .memberships
            .iter()
            .filter(|row| row.standing == "pending" && self.owns(&account_id, &row.table_id))
            .filter_map(|row| {
                let table_name = self.table_name(&row.table_id)?;
                let username = self.username(&row.account_id)?;
                Some(SeatRequestRow {
                    table_id: row.table_id.clone(),
                    table_name,
                    account_id: row.account_id.clone(),
                    username,
                })
            })
            .collect();
        let friends = self
            .file
            .friends
            .iter()
            .filter(|bond| bond.accepted && (bond.from_id == account_id || bond.to_id == account_id))
            .filter_map(|bond| {
                let other = if bond.from_id == account_id { &bond.to_id } else { &bond.from_id };
                Some(PersonRow { id: other.clone(), username: self.username(other)? })
            })
            .collect();
        let incoming = self
            .file
            .friends
            .iter()
            .filter(|bond| !bond.accepted && bond.to_id == account_id)
            .filter_map(|bond| Some(PersonRow { id: bond.from_id.clone(), username: self.username(&bond.from_id)? }))
            .collect();
        let friend_tables = self
            .file
            .open_id
            .as_ref()
            .and_then(|open| self.file.tables.iter().find(|card| &card.id == open))
            .filter(|card| self.friends_with(&account_id, &card.owner_id) && card.owner_id != account_id)
            .and_then(|card| {
                Some(vec![OpenTableRow {
                    id: card.id.clone(),
                    name: card.name.clone(),
                    owner_name: self.username(&card.owner_id)?,
                }])
            })
            .unwrap_or_default();
        Ok(Lobby {
            account,
            tables,
            requests,
            friends,
            incoming,
            friend_tables,
            open_id: self.file.open_id.clone(),
        })
    }

    pub fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let temp = self.path.with_extension("json.tmp");
        if let Ok(file) = fs::File::create(&temp) {
            if serde_json::to_writer_pretty(file, &self.file).is_ok() {
                let _ = fs::rename(&temp, &self.path);
            }
        }
    }

    fn insert_account(&mut self, username: &str, password: &str) -> Account {
        let color = COLORS[self.file.next_color % COLORS.len()].to_string();
        self.file.next_color += 1;
        let account = Account {
            id: Uuid::new_v4().to_string(),
            username: username.to_string(),
            password: hash_password(password),
            color,
            settings: AccountSettings::default(),
        };
        self.file.accounts.push(account.clone());
        account
    }

    fn issue(&mut self, account_id: &str) -> String {
        let token = Uuid::new_v4().to_string();
        self.file.sessions.push(Session { token: token.clone(), account_id: account_id.to_string() });
        token
    }

    fn account_id(&self, token: &str) -> Result<String, String> {
        self.file
            .sessions
            .iter()
            .find(|session| session.token == token)
            .map(|session| session.account_id.clone())
            .ok_or_else(|| "sign in again".to_string())
    }

    fn username(&self, id: &str) -> Option<String> {
        self.file.accounts.iter().find(|account| account.id == id).map(|account| account.username.clone())
    }

    fn member_of(&self, account_id: &str, table_id: &str) -> bool {
        self.file.memberships.iter().any(|row| row.account_id == account_id && row.table_id == table_id && row.standing != "pending")
    }

    fn friends_with(&self, a: &str, b: &str) -> bool {
        self.file.friends.iter().any(|bond| {
            bond.accepted && ((bond.from_id == a && bond.to_id == b) || (bond.from_id == b && bond.to_id == a))
        })
    }
}

fn public_account(account: &Account) -> PublicAccount {
    PublicAccount {
        id: account.id.clone(),
        username: account.username.clone(),
        color: account.color.clone(),
        settings: account.settings.clone(),
    }
}

fn hash_password(password: &str) -> String {
    let salt = Uuid::new_v4().to_string();
    format!("{salt}${}", digest(&salt, password))
}

fn password_matches(stored: &str, password: &str) -> bool {
    let Some((salt, expected)) = stored.split_once('$') else {
        return false;
    };
    digest(salt, password) == expected
}

fn digest(salt: &str, password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

fn clean_username(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.chars().count() < 2 || name.chars().count() > 24 {
        return Err("use a name between 2 and 24 letters".into());
    }
    if !name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
        return Err("use letters, numbers, and dashes in the name".into());
    }
    Ok(name.to_string())
}

fn check_password(password: &str) -> Result<(), String> {
    if password.chars().count() < 4 || password.chars().count() > 80 {
        return Err("use a password of at least 4 characters".into());
    }
    Ok(())
}

fn clean_label(name: &str, fallback: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        return fallback.to_string();
    }
    name.chars().take(48).collect()
}

fn clean_color(color: &str) -> Result<String, String> {
    let color = color.trim();
    let bytes = color.as_bytes();
    if bytes.len() == 7
        && bytes[0] == b'#'
        && bytes[1..].iter().all(|byte| byte.is_ascii_hexdigit())
    {
        return Ok(color.to_ascii_lowercase());
    }
    Err("pick a color from the palette".into())
}

fn clean_settings(settings: AccountSettings) -> AccountSettings {
    let theme = match settings.theme.as_str() {
        "ember" | "night" | "sea" | "hearth" => settings.theme,
        _ => "hearth".into(),
    };
    let dice_tray = if settings.dice_tray == "cover" { "cover" } else { "quarter" };
    let dice_corner = match settings.dice_corner.as_str() {
        "top-left" | "top-right" | "bottom-left" | "bottom-right" => settings.dice_corner,
        _ => "bottom-right".into(),
    };
    AccountSettings {
        theme,
        parchment: clean_color(&settings.parchment).unwrap_or_else(|_| "#f4ecdf".into()),
        gold: clean_color(&settings.gold).unwrap_or_else(|_| "#d4a656".into()),
        dice_tray: dice_tray.into(),
        dice_corner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn house() -> House {
        let dir = std::env::temp_dir().join(format!("game-table-house-{}", Uuid::new_v4()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let (house, fresh) = House::load(&dir, "table", "tetst");
        assert!(fresh);
        house
    }

    #[test]
    fn the_first_boot_keeps_a_host_and_the_open_campaign() {
        let mut house = house();
        let (token, account) = house.login("host", "table").unwrap();
        assert_eq!(account.username, "host");
        assert!(!account_password_is_plain(&house, "table"));
        let lobby = house.lobby(&token).unwrap();
        assert_eq!(lobby.tables.len(), 1);
        assert_eq!(lobby.tables[0].name, "tetst");
        assert_eq!(lobby.open_id.as_deref(), Some(lobby.tables[0].id.as_str()));
    }

    #[test]
    fn approve_once_is_spent_after_a_seat_and_always_is_not() {
        let mut house = house();
        let (host, _) = house.login("host", "table").unwrap();
        let (guest, guest_account) = house.register("mira", "secret").unwrap();
        let table_id = house.lobby(&host).unwrap().open_id.unwrap();
        house.request_seat(&guest, &table_id).unwrap();
        house.approve(&host, &table_id, &guest_account.id, false).unwrap();
        assert!(matches!(house.door(&guest_account.id, &table_id), Door::Once { .. }));
        house.remember_seat(&table_id, &guest_account.id, "seat-1", true);
        match house.door(&guest_account.id, &table_id) {
            Door::Spent { seat } => assert_eq!(seat.as_deref(), Some("seat-1")),
            other => panic!("expected a spent seat, got {other:?}"),
        }

        house.request_seat(&guest, &table_id).unwrap();
        house.approve(&host, &table_id, &guest_account.id, true).unwrap();
        assert!(matches!(house.door(&guest_account.id, &table_id), Door::Always { .. }));
        house.remember_seat(&table_id, &guest_account.id, "seat-2", true);
        assert!(matches!(house.door(&guest_account.id, &table_id), Door::Always { .. }));
    }

    #[test]
    fn friends_can_see_an_open_table() {
        let mut house = house();
        let (host, host_account) = house.login("host", "table").unwrap();
        let (guest, _) = house.register("mira", "secret").unwrap();
        house.friend_request(&host, "mira").unwrap();
        house.accept_friend(&guest, &host_account.id).unwrap();
        let lobby = house.lobby(&guest).unwrap();
        assert_eq!(lobby.friend_tables.len(), 1);
        assert_eq!(lobby.friend_tables[0].owner_name, "host");
        assert!(lobby.friends.iter().any(|person| person.username == "host"));
    }

    fn account_password_is_plain(house: &House, password: &str) -> bool {
        house.file.accounts.iter().any(|account| account.password == password)
    }
}
