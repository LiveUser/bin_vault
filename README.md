# Bin Vault
A custom binary database. Hecho en Puerto Rico por Radamés Jomuel Valentín Reyes.

## Create a database entry point (the folder where all of the database files will be stored)
- Entry
~~~rs
let entry:Entry = Entry { 
    db_path: "./test_database".to_string(),
};
// Create database if it does not exist
entry.create();
// Select entry object
let db_object:DbObject = entry.select();
~~~
## Methods
- Insert value and replace any previous value
~~~rs
db_object.insert_and_replace("text".to_string(), ["Hello World".to_string()].to_vec());
~~~
- Insert to append values
~~~rs
db_object.insert("numbers".to_string(), ["1".to_string()].to_vec());
~~~
- Insert object
~~~rs
let mut map:HashMap<String, Vec<String>> = HashMap::new();
map.insert("some_key".to_string(), ["Some Value".to_string()].to_vec());
db_object.insert_object("objects".to_string(), map.clone());
//Nest a child object inside to test recursive deletion
my_object.insert_object("objects".to_string(), map.clone());
~~~
- Get values
~~~rs
let vals:Vec<String> = db_object.get_values("numbers".to_string());
~~~
- Delete a value
~~~rs
db_object.delete_value("numbers".to_string(), 0);
~~~
- Select child object
~~~rs
let my_object:DbObject = db_object.select_object("objects".to_string(), 0);
~~~
- Determine if object is valid (true is an object, false is just a value)
~~~rs
println!("Object is valid = {}", my_object.is_valid());
~~~
- Delete the parent object (and recursively delete child objects)
~~~rs
db_object.delete_value("objects".to_string(), 0);
~~~
- View object as HashMap
~~~rs
println!("{:?}", db_object.view());
~~~