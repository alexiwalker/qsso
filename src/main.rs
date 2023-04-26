use std::ffi::{ OsString};
use serde::{Serialize, Deserialize};
use std::{env, fs};
use chrono::prelude::*;
fn main() {

    let mut default_region:String = "ap-southeast-2".to_string();


    let h = match home::home_dir() {
        None => {
            "".to_string()
        }
        Some(path) => {
            path.to_str().unwrap().to_string()
        }
    };

    let credentials_file = format!("{h}/.aws/config");
    let defaults_file = format!("{h}/.aws/qsso_default");
    let cli_cache = format!("{h}/.aws/cli/cache");

    let mut chosen_profile = "default".to_string();
    let mut sso_login_command = std::process::Command::new("aws");
    let command = sso_login_command
        .arg("sso")
        .arg("login");


    let clear_cache_result = fs::remove_dir_all(cli_cache.clone());

    match clear_cache_result {
        Ok(_) => {
            fs::create_dir(cli_cache.clone()).unwrap();
        }
        Err(e) => {
            println!("{}",e);
            return;
        }
    }

    let args: Vec<String> = env::args().collect();
    if args.len() == 2 {
        let prof = args[1].clone();
        chosen_profile = prof.clone();
        command
        .arg("--profile")
        .arg(prof.clone());
    }

    if args.len() == 3 && args[1] == "default" {
        let profile = args[2].clone();
        let region = "ap-southeast-2";
        let str = profile+","+region;
        let _ = fs::write(defaults_file, str);
        return;
    }

    if args.len() == 4 && args[1] == "default" {
        let profile = args[2].clone();
        let region = args[3].clone();
        let str = format!("{},{}",profile,region);
        let _ = fs::write(defaults_file, str);
        return;
    }

    if args.len()==1 {
        let default_profile = fs::read_to_string(defaults_file.clone());
        match default_profile {
            Ok(str) => {
                let parts:Vec<&str> = str.split(",").collect();
                chosen_profile = parts[0].clone().to_string();
                default_region=parts[1].clone().to_string();
                println!("Using default profile: {}. Change with sso default <name>", str.clone());
                command
                .arg("--profile")
                .arg(chosen_profile.clone());
            }
            Err(_) => {}
        }
    }

    let local: DateTime<Local> = Local::now();


    let res = command.output().unwrap();
    let out = String::from_utf8(res.stdout);

    println!("{}",out.unwrap().clone());

    let mut aws_configure_call_to_refresh_cache = std::process::Command::new("aws");
    let cache_command = aws_configure_call_to_refresh_cache
        .arg("configure")
        .arg("list")
        .arg("--profile")
        .arg(chosen_profile);
    let _ = cache_command.output().unwrap();

    let directory_contents = fs::read_dir(cli_cache.clone());



    let cached_file_to_use = match directory_contents {
        Ok(contents) => {
            let mut lowest_time = 0 as u128;
            let mut file_path:OsString = OsString::new();
            for x in contents.into_iter() {
                match x {
                    Ok(dir_entry) => {
                        let name = dir_entry.file_name().clone();
                        let modified = dir_entry.metadata().unwrap().modified().unwrap();
                        let modified_time = modified.elapsed().unwrap().as_millis();
                        if lowest_time ==0 || modified_time < lowest_time {
                            lowest_time = modified_time;
                            file_path = name.clone();

                            if lowest_time>1000 {
                                println!("used cache file appears to be old, consider rerunning")
                            }
                        }
                    }
                    Err(err) => {
                        println!("{}",err);
                        // Err(())
                    }
                }
            }
            Ok(cli_cache.clone()+"/"+ file_path.to_str().unwrap())

        }
        Err(err) => {
            println!("{}",err);
            Err(())
        }
    };

    match cached_file_to_use {
        Ok(file_name) => {

            let serialized = fs::read_to_string(file_name).unwrap();
            let sso_credentials: Root = serde_json::from_str(&serialized).unwrap();
            let creds = sso_credentials.credentials;

            let credentials_file_content = fs::read_to_string(credentials_file.to_string());

            let key_string = format!("aws_access_key_id={}", creds.access_key_id);
            let secret_string = format!("aws_secret_access_key={}", creds.secret_access_key);
            let session_string = format!("aws_session_token={}", creds.session_token);
            let region_string = format!("region={}", default_region);

            let profile_string = format!("\
            {key_string}\r\n\
            {secret_string}\r\n\
            {session_string}\r\n\
            {region_string}\r\n\
            #automatically updated by qsso at {}\r\n\
            ", local);
            // println!("credString: \r\n{}", profileString);

            let mut new_file:Vec<String> = vec![];

            match credentials_file_content {
                Ok(string) => {

                    let lines = string.lines();
                    let mut is_default_profile = false;
                    for line in lines {
                        if line == "[default]" {
                            new_file.push(line.to_string());
                            new_file.push(profile_string.clone());
                            is_default_profile =true;
                        } else if line.starts_with("[") && line.ends_with("]"){
                            new_file.push(line.to_string());
                            is_default_profile =false
                        } else if !is_default_profile {
                            new_file.push(line.to_string())
                        }
                        //skip lines from the default profile until we reach a new profile
                    }

                }
                Err(err) => {
                    println!("{}",err);
                    return;
                }
            }

            let full_string = new_file.join("\r\n");

            let success = fs::write(credentials_file, full_string);

            match success {
                Ok(_) => {}
                Err(err) => {
                    println!("could not update credentials file: {}", err)
                }
            }

        }
        Err(_) => {
            println!("An error occurred")
        }
    }

}



#[derive(Serialize, Deserialize)]
struct Credentials {
    #[serde(rename = "AccessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "SecretAccessKey")]
    pub secret_access_key: String,
    #[serde(rename = "SessionToken")]
    pub session_token: String,
    #[serde(rename = "Expiration")]
    pub expiration: String,
}

#[derive(Serialize, Deserialize)]
struct Root {
    #[serde(rename = "ProviderType")]
    pub provider_type: String,
    #[serde(rename = "Credentials")]
    pub credentials: Credentials,
}