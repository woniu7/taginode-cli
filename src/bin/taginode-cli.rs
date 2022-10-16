use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Error, ErrorKind};
use std::os::unix::prelude::MetadataExt;
use std::time::UNIX_EPOCH;
use sqlite3::Connection;
use taginode::INode;

fn usage() {
    eprintln!("Usage: taginode-cli [option] tag <file> <tag> [tag1,tag2...]");
    eprintln!("Usage: taginode-cli [option] search [tag1,tag2...]");
    eprintln!("Usage: taginode-cli [option] list tags");
    eprintln!(
"Option: 
        -f <db>    Specify db path to store data, default ~/.taginode.db
"   );
    std::process::exit(1);
}

fn main() -> Result<(), Error>{
    let args: Vec<String> = env::args().collect();
    let args = &args[1..];

    let mut options: HashMap<u8, &str> = HashMap::new();
    let mut operands: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "-f" => {
                if i+1 >= args.len() {
                    eprint!("Option -f need option-argument");
                    usage();
                }
                options.insert(b'f', args[i+1].as_str());
                i += 1;
            },
            "--" => {
                for e in &args[(i+1)..] {
                    operands.push(e);
                }
                break;
            }
            other => operands.push(other),
        }
        i += 1;
    } 
    if operands.is_empty() {
        usage();
    }

    let mut h = env::var("HOME").unwrap();
    h.push_str("/.taginode.db");
    let default_db_path = h.as_str();
    let db_path = options.get(&b'f').unwrap_or(&default_db_path);
    let db = taginode::sql::init(&db_path);

    match operands[0] {
        "tag" => tag(&operands[1..], db),
        "search" => search(&operands[1..], db),
        "list" => list(&operands[1..], db),
        _ => usage(),
    }
    Ok(())
}

fn tag(args: &[&str], db: Connection) {
    if args.len() < 2 {
        usage();
    }
    let files = &args[0..1];
    let tag_names: Vec<&str> = args[1].split(",").collect();
    eprintln!("tag_names: {:?}, files: {:?}", tag_names, files);

    for file in files {
        let metadata = fs::metadata(file.to_string());
        let metadata = match metadata {
            Ok(metadata) => metadata,
            Err(error) => {
                eprintln!("{:?}", error);
                continue;
            },
        };
        let btime = metadata.created();
        let btime = match btime {
            Ok(btime) => { 
                match btime.duration_since(UNIX_EPOCH) {
                    Ok(btime) => Some(btime.as_secs()),
                    Err(error) => {
                        eprintln!("Warning: {:?}", error);
                        None
                    }
                }
            },
            Err(error) => {
                eprintln!("Warning: {:?}", error);
                None
            }
        };
        taginode::add(&db, 
            &vec![ INode{ device: metadata.dev(), number: metadata.ino(), btime } ],
            &tag_names,
        );
    }
}

fn search(args: &[&str], db: Connection) {
    if args.len() != 1 {
        usage();
    }
    let tag_names: Vec<&str> = args[0].split(",").collect();
    let paths = vec!["."];
    eprintln!("tag_names: {:?}, paths: {:?}", tag_names, paths);

    let inodes = taginode::get_inodes(&db, &tag_names);
    let mut dev_inode_map: HashMap<u64, HashMap<u64, &INode>> = HashMap::new();
    for inode in &inodes {
        let inode_map = dev_inode_map.get_mut(&inode.device);
        match inode_map {
            Some(inode_map) => {
                inode_map.insert(inode.number, inode);
            }, 
            None => {
                let mut inode_map = HashMap::new();
                inode_map.insert(inode.number, inode);
                dev_inode_map.insert(inode.device, inode_map);
            }
        };
    }
    for path in paths {
        match process_file(&dev_inode_map, path) {
            Err(error) => eprintln!("{error:?}"),
            _ => (),
        }
    }
}

fn process_file(dev_inode_map: &HashMap<u64, HashMap<u64, &INode>>, f: &str) -> Result<(), Error> {
    let metadata = fs::metadata(f)?;
    if dev_inode_map.get(&metadata.dev()).is_some() {
        let inode_map = dev_inode_map.get(&metadata.dev()).unwrap();
        let mut hit = false;
        match inode_map.get(&metadata.ino()) {
            None => (),
            Some(ino) => {
                let created = (|| -> Result<u64, Error> {
                    match metadata.created()?.duration_since(UNIX_EPOCH) {
                        Ok(btime) => 
                        Ok(btime.as_secs()),
                        Err(error) => 
                        Err(Error::new(ErrorKind::Other, error.to_string())),
                    }
                })();
                if ino.btime == None || created.is_err() || ino.btime.unwrap() == created.unwrap() {
                    hit = true;
                }
            }
        };
        if hit {
            println!("{} ", f);
        }
        if metadata.is_dir() {
            let paths = fs::read_dir(f)?;
            for path in paths {
                match path {
                    Ok(entry) => {
                        if entry.metadata()?.is_symlink() {
                            continue;
                        }
                        let p = entry.path();
                        let p= p.to_str().unwrap();
                        match process_file(&dev_inode_map, p) {
                            Err(error) => eprintln!("{p:?} {error:?}"),
                            _ => (),
                        }
                    }
                    Err(error) => eprintln!("{f} {error:?}"),
                };
            }
        }
    }
    Ok(())
}

fn list(args: &[&str], db: Connection) {
    if args.len() < 1 || args[0] != "tags" {
        usage();
    }

    let tag_names = taginode::list_tags(&db);
    println!("{tag_names:?}")
}
