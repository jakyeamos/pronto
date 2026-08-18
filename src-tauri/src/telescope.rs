use crate::behavior_assurance::{
    BehaviorContract, BehaviorContractBehavior, CONTRACT_PATH, CONTRACT_SCHEMA,
};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, LazyLock, Mutex,
};

include!("telescope/part-01.rs");
include!("telescope/part-02.rs");
include!("telescope/part-03.rs");
include!("telescope/part-04.rs");
include!("telescope/part-05.rs");
include!("telescope/part-06.rs");
include!("telescope/part-07.rs");
include!("telescope/part-08.rs");
include!("telescope/part-09.rs");
include!("telescope/part-10.rs");

#[cfg(test)]
mod tests {
    include!("telescope/tests-01.rs");
    include!("telescope/tests-02.rs");
    include!("telescope/tests-03.rs");
}
