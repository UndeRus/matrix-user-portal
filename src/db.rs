use std::sync::Arc;

use rand::{Rng, distr::Alphanumeric, rng};
use rocksdb::{ColumnFamilyDescriptor, DB, IteratorMode, Options};

pub fn open_db() -> DB {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);

    let cf_invites = ColumnFamilyDescriptor::new("invites", Options::default());

    let db = DB::open_cf_descriptors(&opts, "portaldb", vec![cf_invites]).unwrap();

    return db;
}

#[derive(Clone)]
pub struct InvitesRepo {
    db: Arc<DB>,
}

impl InvitesRepo {
    pub fn new() -> Self {
        Self {
            db: Arc::new(open_db()),
        }
    }

    pub fn new_invite(&self) -> String {
        let invite_code = new_invite();
        let db = self.db.clone();
        let cf = db.cf_handle("invites").unwrap();
        db.put_cf(&cf, format!("INVITE:{invite_code}"), &[0])
            .unwrap();
        invite_code
    }

    pub fn active_invites(&self) -> String {
        let db = self.db.clone();
        let cf = db.cf_handle("invites").unwrap();

        let iter = db.iterator_cf(&cf, IteratorMode::Start);

        let mut result = vec![];
        for item in iter {
            let (k, v) = item.unwrap();
            if v.as_ref() == &[0] {
                result.push(k);
            }
        }

        let results = result
            .iter()
            .map(|f| String::from_utf8_lossy(f) + "\n")
            .collect();
        results
    }

    pub fn use_invite(&self, invite_code: &str) {
        let db = self.db.clone();
        let cf: Arc<rocksdb::BoundColumnFamily<'_>> = db.cf_handle("invites").unwrap();
        db.put_cf(&cf, format!("INVITE:{}", invite_code), &[1])
            .unwrap();
    }

    pub fn check_invite(&self, invite_code: &str) -> Option<bool> {
        let db = self.db.clone();
        let cf: Arc<rocksdb::BoundColumnFamily<'_>> = db.cf_handle("invites").unwrap();

        if let Ok(Some(invite_is_active)) = db.get_cf(&cf, format!("INVITE:{}", invite_code)) {
            if invite_is_active == &[1] {
                return Some(false);
            } else {
                return Some(true);
            }
        }
        None
    }
}

const STRING_LEN: usize = 8;

fn new_invite() -> String {
    let random_string: String = rng()
        .sample_iter(&Alphanumeric)
        .take(STRING_LEN)
        .map(char::from) // Convert the u8 samples to chars
        .collect();
    random_string
}
