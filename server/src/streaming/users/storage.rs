use crate::streaming::storage::{Storage, UserStorage};
use crate::streaming::users::pat::PersonalAccessToken;
use crate::streaming::users::user::User;
use async_trait::async_trait;
use iggy::error::Error;
use iggy::models::user_info::UserId;
use sled::Db;
use std::str::from_utf8;
use std::sync::Arc;
use tracing::{error, info};

const USERS_KEY_PREFIX: &str = "users";
const PAT_KEY_PREFIX: &str = "pat";

#[derive(Debug)]
pub struct FileUserStorage {
    db: Arc<Db>,
}

impl FileUserStorage {
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }
}

unsafe impl Send for FileUserStorage {}
unsafe impl Sync for FileUserStorage {}

#[async_trait]
impl UserStorage for FileUserStorage {
    async fn load_by_id(&self, id: UserId) -> Result<User, Error> {
        let mut user = User::empty(id);
        self.load(&mut user).await?;
        Ok(user)
    }

    async fn load_by_username(&self, username: &str) -> Result<User, Error> {
        let user_id_key = get_user_id_key(username);
        let user_id = self.db.get(&user_id_key);
        if user_id.is_err() {
            return Err(Error::CannotLoadResource(user_id_key));
        }

        let user_id = user_id.unwrap();
        if user_id.is_none() {
            return Err(Error::CannotLoadResource(user_id_key));
        }

        let user_id = user_id.unwrap();
        let user_id = u32::from_le_bytes(user_id.as_ref().try_into()?);
        let mut user = User::empty(user_id);
        self.load(&mut user).await?;
        Ok(user)
    }

    async fn load_all_users(&self) -> Result<Vec<User>, Error> {
        let mut users = Vec::new();
        for data in self.db.scan_prefix(format!("{}:", USERS_KEY_PREFIX)) {
            let user = match data {
                Ok((_, value)) => match rmp_serde::from_slice::<User>(&value) {
                    Ok(user) => user,
                    Err(err) => {
                        error!("Cannot deserialize user. Error: {}", err);
                        return Err(Error::CannotDeserializeResource(
                            USERS_KEY_PREFIX.to_string(),
                        ));
                    }
                },
                Err(err) => {
                    error!("Cannot load user. Error: {}", err);
                    return Err(Error::CannotLoadResource(USERS_KEY_PREFIX.to_string()));
                }
            };
            users.push(user);
        }

        Ok(users)
    }

    async fn load_all_pats(&self) -> Result<Vec<PersonalAccessToken>, Error> {
        let mut pats = Vec::new();
        for data in self.db.scan_prefix(format!("{}:token:", PAT_KEY_PREFIX)) {
            let pat = match data {
                Ok((_, value)) => match rmp_serde::from_slice::<PersonalAccessToken>(&value) {
                    Ok(pat) => pat,
                    Err(err) => {
                        error!("Cannot deserialize PAT. Error: {}", err);
                        return Err(Error::CannotDeserializeResource(PAT_KEY_PREFIX.to_string()));
                    }
                },
                Err(err) => {
                    error!("Cannot load PAT. Error: {}", err);
                    return Err(Error::CannotLoadResource(PAT_KEY_PREFIX.to_string()));
                }
            };
            pats.push(pat);
        }

        Ok(pats)
    }

    async fn load_pats_for_user(&self, user_id: UserId) -> Result<Vec<PersonalAccessToken>, Error> {
        let mut pats = Vec::new();
        let key = format!("{}:user:{}:", PAT_KEY_PREFIX, user_id);
        for data in self.db.scan_prefix(&key) {
            match data {
                Ok((_, value)) => {
                    let token = from_utf8(&value)?;
                    let pat = self.load_pat_by_token(token).await?;
                    pats.push(pat);
                }
                Err(err) => {
                    error!("Cannot load PAT. Error: {}", err);
                    return Err(Error::CannotLoadResource(key));
                }
            };
        }

        Ok(pats)
    }

    async fn load_pat_by_token(&self, token: &str) -> Result<PersonalAccessToken, Error> {
        let key = get_pat_key(token);
        let pat = self.db.get(&key);
        if pat.is_err() {
            return Err(Error::CannotLoadResource(key));
        }

        let pat = pat.unwrap();
        if pat.is_none() {
            return Err(Error::CannotLoadResource(key));
        }

        let pat = pat.unwrap();
        let pat = rmp_serde::from_slice::<PersonalAccessToken>(&pat);
        if pat.is_err() {
            return Err(Error::CannotDeserializeResource(key));
        }

        let pat = pat.unwrap();
        Ok(pat)
    }

    async fn load_pat_by_name(
        &self,
        user_id: UserId,
        name: &str,
    ) -> Result<PersonalAccessToken, Error> {
        let key = get_pat_name_key(user_id, name);
        let token = self.db.get(&key);
        if token.is_err() {
            return Err(Error::CannotLoadResource(key));
        }

        let token = token.unwrap();
        if token.is_none() {
            return Err(Error::CannotLoadResource(key));
        }

        let token = token.unwrap();
        let token = from_utf8(&token);
        if token.is_err() {
            return Err(Error::CannotDeserializeResource(key));
        }

        let token = token.unwrap();
        self.load_pat_by_token(token).await
    }

    async fn save_pat(&self, pat: &PersonalAccessToken) -> Result<(), Error> {
        let key = get_pat_key(&pat.token);
        match rmp_serde::to_vec(&pat) {
            Ok(data) => {
                if let Err(err) = self.db.insert(&key, data) {
                    error!(
                        "Cannot save PAT for user with ID: {}. Error: {}",
                        pat.user_id, err
                    );
                    return Err(Error::CannotSaveResource(key.to_string()));
                }
                if let Err(err) = self.db.insert(
                    get_pat_name_key(pat.user_id, &pat.name),
                    pat.token.as_bytes(),
                ) {
                    error!(
                        "Cannot save PAT for user with ID: {}. Error: {}",
                        pat.user_id, err
                    );
                    return Err(Error::CannotSaveResource(key.to_string()));
                }
            }
            Err(err) => {
                error!(
                    "Cannot serialize PAT for user with ID: {}. Error: {}",
                    pat.user_id, err
                );
                return Err(Error::CannotSerializeResource(key));
            }
        }

        info!("Saved PAT for user with ID: {}.", pat.user_id);
        Ok(())
    }

    async fn delete_pat(&self, user_id: UserId, name: &str) -> Result<(), Error> {
        let pat = self.load_pat_by_name(user_id, name).await?;
        info!("Deleting PAT with name: {name} for user with ID: {user_id}...");
        let key = get_pat_name_key(user_id, name);
        if self.db.remove(&key).is_err() {
            return Err(Error::CannotDeleteResource(key));
        }
        let key = get_pat_key(&pat.token);
        if self.db.remove(&key).is_err() {
            return Err(Error::CannotDeleteResource(key));
        }
        info!("Deleted PAT with name: {name} for user with ID: {user_id}.");
        Ok(())
    }
}

#[async_trait]
impl Storage<User> for FileUserStorage {
    async fn load(&self, user: &mut User) -> Result<(), Error> {
        let key = get_user_key(user.id);
        let user_data = self.db.get(&key);
        if user_data.is_err() {
            return Err(Error::CannotLoadResource(key));
        }

        let user_data = user_data.unwrap();
        if user_data.is_none() {
            return Err(Error::CannotLoadResource(key));
        }

        let user_data = user_data.unwrap();
        let user_data = rmp_serde::from_slice::<User>(&user_data);
        if user_data.is_err() {
            return Err(Error::CannotDeserializeResource(key));
        }

        let user_data = user_data.unwrap();
        user.status = user_data.status;
        user.username = user_data.username;
        user.password = user_data.password;
        user.created_at = user_data.created_at;
        user.permissions = user_data.permissions;
        Ok(())
    }

    async fn save(&self, user: &User) -> Result<(), Error> {
        let key = get_user_key(user.id);
        match rmp_serde::to_vec(&user) {
            Ok(data) => {
                if let Err(err) = self.db.insert(&key, data) {
                    error!("Cannot save user with ID: {}. Error: {}", user.id, err);
                    return Err(Error::CannotSaveResource(key.to_string()));
                }
                if let Err(err) = self
                    .db
                    .insert(get_user_id_key(&user.username), &user.id.to_le_bytes())
                {
                    error!(
                        "Cannot save username for user with ID: {}. Error: {}",
                        user.id, err
                    );
                    return Err(Error::CannotSaveResource(key.to_string()));
                }
            }
            Err(err) => {
                error!("Cannot serialize user with ID: {}. Error: {}", user.id, err);
                return Err(Error::CannotSerializeResource(key));
            }
        }

        info!("Saved user with ID: {}.", user.id);
        Ok(())
    }

    async fn delete(&self, user: &User) -> Result<(), Error> {
        info!("Deleting user with ID: {}...", user.id);
        let key = get_user_key(user.id);
        if self.db.remove(&key).is_err() {
            return Err(Error::CannotDeleteResource(key));
        }
        let key = get_user_id_key(&user.username);
        if self.db.remove(&key).is_err() {
            return Err(Error::CannotDeleteResource(key));
        }
        info!("Deleted user with ID: {}.", user.id);
        Ok(())
    }
}

fn get_user_key(user_id: UserId) -> String {
    format!("{}:{}", USERS_KEY_PREFIX, user_id)
}

fn get_user_id_key(username: &str) -> String {
    format!("{}_id:{}", USERS_KEY_PREFIX, username)
}

fn get_pat_key(token_hash: &str) -> String {
    format!("{}:token:{}", PAT_KEY_PREFIX, token_hash)
}

fn get_pat_name_key(user_id: UserId, name: &str) -> String {
    format!("{}:user:{}:{}", PAT_KEY_PREFIX, user_id, name)
}
