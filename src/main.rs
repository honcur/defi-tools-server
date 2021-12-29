use std::env;
use std::ops::{Div,Add};
use std::str::FromStr;
use web3::contract::{Contract, Options};
use web3::transports::Http;
use web3::types::{Address};
use web3::Web3;
use chrono::prelude::*;
use std::time::Duration;
use std::thread;

use web3::contract::Error;
use redis::{self, Commands,  Connection};
use std::sync::{Arc};


use defi_tools_serve::{thread_pool};
static mut SHUTDOWN: bool = false;

#[tokio::main]
async fn main() -> web3::Result<()> {
    dotenv::dotenv().ok();
    //borrowRatePerBlock
    //supplyRatePerBlock
   
    let pool = thread_pool::ThreadPool::new(6);
    pool.execute(|| {
        let mut input = String::from("");
        while !input.trim().eq_ignore_ascii_case("exit") {
            std::io::stdin().read_line(&mut input)
            .expect("Failed to read line");
            println!("-------{}-------", input.trim());
        }
        unsafe {
            println!("unsafe input = {}", input);
            SHUTDOWN = true;
        }
    }); 
    let redis_url = "redis://127.0.0.1:6379/";
    let  client = redis::Client::open(&redis_url[..]).expect("redis连接错误");
   
   
    let websocket = web3::transports::Http::new(&env::var("RPC_URL").unwrap())?;
    let web3 = web3::Web3::new(websocket);
    let web3 = Arc::new(web3);
    while !is_shutdown() {
        let mut p_con = match client.get_connection() {
            Ok(rs) => rs,
            Err(_) => client.get_connection().expect("连接redis错误")
        };
        let web3 = Arc::clone(& web3);
        pool.execute( move || {
            //task(&mut con);
            let rt = tokio::runtime::Runtime::new().unwrap(); 
            rt.block_on(task(&mut p_con, &web3) );
            println!("----------------" );
            
        });
        thread::sleep(Duration::from_secs(60));
    }
    Ok(())
}

fn is_shutdown() -> bool {
    unsafe {
        SHUTDOWN
    }
}

async fn task(con:  &mut Connection, web3s: &Web3<Http>) {
    let borrow_rate_per_block = get_rate_pre_block(web3s, "borrowRatePerBlock").await.expect("msg");
    let borrow_apy = calaute_apy(borrow_rate_per_block);
    let borrow_apy = (borrow_apy * 1e18) as u64;

    let supply_rate_per_block = get_rate_pre_block(web3s, "supplyRatePerBlock").await.expect("msg");
    let supply_apy = calaute_apy(supply_rate_per_block);
    let supply_apy = (supply_apy * 1e18) as u64;

    let dt = Local::now();
    let timestamp = &dt.timestamp().to_string()[..];
   
    if borrow_rate_per_block > 0 {
        let mut borrow_apy_key = String::from("bnb_borrow_apy_");
        borrow_apy_key.push_str(timestamp);
        let _:Result<(), redis::RedisError> =  con.set(&borrow_apy_key, borrow_apy);
        
    }
    
    if supply_rate_per_block > 0 {
        let mut supply_apy_key = String::from("bnb_supply_apy_");
        supply_apy_key.push_str(timestamp);
        let _:Result<(), redis::RedisError> =  con.set(&supply_apy_key, borrow_apy);
    }
    println!("borrow_apy = {} supply_apy = {}", borrow_apy, supply_apy);
   
    
}

fn calaute_apy(rate: u128) -> f64{
    let ten:u128 = 10;
    let day:u128 = 20 * 60 * 24;
    let mantissa = (ten.pow(18)) as f64;
    let year = 365;
    //let apy = Math.pow(Rate / Mantissa * Day + 1, Year - 1) - 1;
    let day_profilt =  (rate * day) as f64;
    let day_profilt = day_profilt.div(mantissa).add(1.0);
    let apy =  day_profilt.powi(year - 1) - 1.0;
    apy
}

async fn get_rate_pre_block(web3s: &Web3<Http>, method: &str) -> web3::Result<u128> {
    let vbnb_addr = Address::from_str("0xa07c5b74c9b40447a954e1466938b865b6bbea36").unwrap();
    let token_contract =
        Contract::from_json(web3s.eth(), vbnb_addr, include_bytes!("vbnb_abi.json")).unwrap();
        let result:Result<u128, Error> =  token_contract
        .query(method, (), None, Options::default(), None)
            .await;
    let rate_per_block: u128 = match  result {
        Ok(res) => res,
        Err(_) => 0
    };
    Ok(rate_per_block)
}