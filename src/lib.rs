use dart_io::{File};
use std::collections::{HashMap};
use bincode::{serialize, deserialize};
use power_plant::{self, generate_unique_token};

pub struct Entry{
    pub db_path: String,
}

impl Entry {
    pub fn create(&self) -> bool{
        let entry_file:File = File{
            full_path: format!("{}/entry.bin", &self.db_path),
        };
        if entry_file.exists(){
            false
        }else {
            entry_file.create_sync();
            // BUG FIX #1: Was HashMap<String, String> — mismatched with the
            // HashMap<String, Vec<String>> that view() deserializes into,
            // causing a panic on every read after creation.
            let map:HashMap<String, Vec<String>> = HashMap::new();
            let bytes: Vec<u8> = serialize(&map).expect("Failed to serialize map");
            entry_file.write_as_bytes(bytes);
            true
        }
    }
    pub fn select(&self) -> DbObject{
        DbObject { 
            db_path: self.db_path.clone(),
            uuid: "entry".to_string(),
        }
    }
}

pub struct DbObject{
    pub db_path:String,
    pub uuid:String,
}
impl DbObject {
    pub fn view(&self) -> HashMap<String, Vec<String>>{
        let file_path:String = format!("{}/{}.bin",&self.db_path,&self.uuid);
        let file:File = File { 
            full_path: file_path,
        };
        let bytes: Vec<u8> = file.read_as_bytes();
        let map:HashMap<String, Vec<String>> = deserialize(&bytes).unwrap_or_default();
        map
    }

    pub fn insert_and_replace(&self, key:String, value:Vec<String>) {
        let mut map:HashMap<String, Vec<String>> = self.view();
        map.insert(key, value);
        let bytes: Vec<u8> = serialize(&map).expect("Failed to serialize map");
        let file_path:String = format!("{}/{}.bin",&self.db_path, &self.uuid);
        let file:File = File { 
            full_path: file_path,
        };
        file.write_as_bytes(bytes);
    }

    // Insert (appends values to existing key, or creates the key if absent)
    pub fn insert(&self, key:String, value:Vec<String>) {
        let map:HashMap<String, Vec<String>> = self.view();
        // BUG FIX #2: Previously did nothing (returned unit) when the key was
        // absent. This silently dropped all inserts on brand-new keys, breaking
        // insert_object's reference storage for any key used for the first time.
        match map.get(&key) {
            None => {
                // Key does not exist yet — insert it with the given values.
                self.insert_and_replace(key, value);
            },
            Some(result) => {
                let mut array:Vec<String> = result.clone();
                for val in value {
                    array.push(val);
                }
                self.insert_and_replace(key, array);
            },
        }
    }

    fn generate_unique_uuid(&self) -> u128 {
        let mut unique_uuid:u128 = 0;
        let mut is_unique = false;
        // BUG FIX #5 (minor / clippy): `while is_unique != true` replaced with
        // the idiomatic `while !is_unique`.
        while !is_unique {
            unique_uuid = generate_unique_token();
            let file_path:String = format!("{}/{}.bin",&self.db_path, unique_uuid);
            let file:File = File { 
                full_path: file_path,
            };
            is_unique = !file.exists();
        }
        unique_uuid
    }

    // Insert object (creates a child file and stores its UUID in the key's list)
    pub fn insert_object(&self, key:String, value:HashMap<String, Vec<String>>) {
        // Create child file
        let unique_uuid:u128 = self.generate_unique_uuid();
        let bytes: Vec<u8> = serialize(&value).expect("Failed to serialize map");
        let file_path:String = format!("{}/{}.bin",&self.db_path, unique_uuid);
        let file:File = File { 
            full_path: file_path,
        };
        file.create_sync();
        file.write_as_bytes(bytes);
        // Store reference (depends on BUG FIX #2 to work for new keys)
        self.insert(key, [unique_uuid.to_string()].to_vec());
    }

    // Get values for a key
    pub fn get_values(&self, key:String) -> Vec<String> {
        let map:HashMap<String, Vec<String>> = self.view();
        map.get(&key).cloned().unwrap_or_default()
    }

    // Select a child DbObject by key and index
    pub fn select_object(&self, key:String, index:usize) -> DbObject {
        let values:Vec<String> = self.get_values(key);
        match values.get(index) {
            None => {
                DbObject { 
                    db_path: self.db_path.clone(), 
                    uuid: "".to_string(),
                }
            },
            Some(uuid) => {
                let file:File = File { 
                    full_path: format!("{}/{}.bin", self.db_path, uuid),
                };
                if file.exists() {
                    DbObject { 
                        db_path: self.db_path.clone(), 
                        uuid: uuid.clone(),
                    }
                } else {
                    // Return empty uuid since file is invalid
                    DbObject { 
                        db_path: self.db_path.clone(), 
                        uuid: "".to_string(),
                    }
                }
            }
        }
    }

    // Is valid — checks that this DbObject points to a real file
    pub fn is_valid(&self) -> bool {
        // BUG FIX #3 (minor): Use is_empty() instead of != "" (idiomatic Rust,
        // avoids a clippy warning).
        !self.uuid.is_empty()
    }

    // Delete value at index under key; recursively deletes child objects first
    pub fn delete_value(&self, key:String, index:usize) {
        let mut values:Vec<String> = self.get_values(key.clone());
        // Check if the value at this index is a child object
        let object:DbObject = self.select_object(key.clone(), index);
        if object.is_valid() {
            // BUG FIX #4: Previously iterated directly over object.view().keys()
            // while calling object.delete_value() inside the loop. Each
            // delete_value call rewrites the file, so the key snapshot from the
            // first view() call became stale and some keys could be skipped.
            // Fix: collect all keys into a Vec before the loop.
            let keys: Vec<String> = object.view().keys().cloned().collect();
            for this_key in keys {
                // Re-query the length on every iteration: each delete_value call
                // rewrites the file, so a count captured before the loop goes stale
                // and the inner loop either under-deletes or panics on remove(0)
                // against an already-empty list.
                while object.get_values(this_key.clone()).len() > 0 {
                    object.delete_value(this_key.clone(), 0);
                }
            }
            // Delete the child file itself
            let file:File = File { 
                full_path: format!("{}/{}.bin", self.db_path, object.uuid),
            };
            file.delete_sync();
        }
        // Remove the reference (or plain value) from this object's list
        values.remove(index);
        self.insert_and_replace(key, values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_methods() {
        // cargo test -- --nocapture
        let entry:Entry = Entry { 
            db_path: "./test_database".to_string(),
        };
        // Create database if it does not exist
        entry.create();
        // Select entry object
        let db_object:DbObject = entry.select();
        // Insert value and replace any previous value
        db_object.insert_and_replace("text".to_string(), ["Hello World".to_string()].to_vec());
        // Insert to append values
        db_object.insert("numbers".to_string(), ["1".to_string()].to_vec());
        db_object.insert("numbers".to_string(), ["2".to_string()].to_vec());
        // Insert object
        let mut map:HashMap<String, Vec<String>> = HashMap::new();
        map.insert("some_key".to_string(), ["Some Value".to_string()].to_vec());
        db_object.insert_object("objects".to_string(), map.clone());
        // Get values
        let vals:Vec<String> = db_object.get_values("numbers".to_string());
        println!("{:?}", vals);
        // Delete a value
        db_object.delete_value("numbers".to_string(), 0);
        // Select child object
        let my_object:DbObject = db_object.select_object("objects".to_string(), 0);
        // Nest a child object inside to test recursive deletion
        my_object.insert_object("objects".to_string(), map.clone());
        // Is valid
        println!("Object is valid = {}", my_object.is_valid());
        // Delete the parent object (should recursively clean up the nested one)
        db_object.delete_value("objects".to_string(), 0);

        println!("{:?}", db_object.view());
    }
}