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

#[cfg(test)]
mod tests {
    include!("telescope/tests-01.rs");
}
