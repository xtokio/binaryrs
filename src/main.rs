use clap::{Arg, App};
use tungstenite::{connect, Message};
use url::Url;
use serde_json;
use tabled::{Tabled, Table, Style};
use colored::*;

use chrono::prelude::DateTime;
use chrono::Local;
use std::time::{UNIX_EPOCH, Duration};

#[derive(Tabled)]
struct Contract {
  contract_id: String,
  contract_type: String,
  entry_price: String,
  exit_price: String,
  entry_time: String,
  exit_time: String,
  amount: String,
  profit: String
}

fn render_table(data: &Vec<Contract>){
  // Clears screen
  print!("{esc}c", esc = 27 as char);

  let table = Table::new(data).with(Style::modern());
  println!("{}",table);
}

fn format_date(unix_epoch: u64) -> String{
  // Creates a new SystemTime from the specified number of whole seconds
  let d = UNIX_EPOCH + Duration::from_secs(unix_epoch);
  // Create DateTime from SystemTime
  let datetime = DateTime::<Local>::from(d);
  // Formats the combined date and time with the specified format string.
  let timestamp_str = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

  return timestamp_str;
}

fn main() {
  // Get parameters from command line
  let matches = App::new("Binaryrs")
        .version("0.1.0")
        .author("Luis Gomez <xtokio@gmail.com>")
        .about("Simple trading bot for binary.com")
        .arg(Arg::new("token")
                 .short('t')
                 .long("token")
                 .takes_value(true)
                 .help("Token value"))
        .arg(Arg::new("app")
                 .short('a')
                 .long("app")
                 .takes_value(true)
                 .help("App ID value"))
        .get_matches();

  let token = matches.value_of("token").unwrap_or("");
  let app = matches.value_of("app").unwrap_or("");

  let ws_url = format!("wss://ws.binaryws.com/websockets/v3?app_id={}",app);

  let mut data_table = vec![];

  let trade_amount : i32 = 1;
  let mut balance : String;
  let mut contract_id : String;
  let mut entry_tick_value : String;
  let mut entry_tick_time : String;
  let mut exit_tick_value : String;
  let mut exit_tick_time : String;
  let mut buy_price : String;
  let mut profit : String;

  let mut track_profit : f32 = 0.0;
  let mut martingale : i32 = 0;

  // Initializes contract_id and makes sure that martingale is used to prevent compiler warnings.
  contract_id = martingale.to_string();
  balance = "".to_string();

  // Connect to the WS server  
  let (mut socket, _response) = connect(Url::parse(&String::from(ws_url)).unwrap()).expect("Can't connect");
  println!("Connected to the server");
  
  let auth = serde_json::json!({ "authorize": token });
  socket.write_message(Message::Text(auth.to_string())).unwrap();

  // Setup Contract
  let mut contract = serde_json::json!({
    "buy": 1,
    "subscribe": 1,
    "price": trade_amount,
    "parameters": { 
      "amount": trade_amount, 
      "basis": "stake", 
      "contract_type": "DIGITEVEN", 
      "currency": "USD", 
      "duration": 1, 
      "duration_unit": "t", 
      "symbol": "R_100" }
  });
  
  // Loop forever, handling parsing each message
  loop {
    let msg = socket.read_message().expect("Error reading message");
    let msg = match msg {
      tungstenite::Message::Text(s) => { s }
      _ => { panic!() }
    };
    let response: serde_json::Value = serde_json::from_str(&msg).expect("Can't parse to JSON");

    if response["msg_type"] == "authorize"{
      // Ask for balance
      let balance_params = serde_json::json!({ "balance": 1, "subscribe": 1 });
      socket.write_message(Message::Text(balance_params.to_string())).unwrap();

      // Buys Contract
      socket.write_message(Message::Text(contract.to_string())).unwrap();
    }

    // Balance
    if response["msg_type"] == "balance"{
      balance = response["balance"]["balance"].to_string().green().to_string();
    }

    // Buy
    if response["msg_type"] == "buy"{
      contract_id = response["buy"]["contract_id"].to_string();
    }

    if response["msg_type"] == "proposal_open_contract"{
      if response["proposal_open_contract"]["is_sold"].to_string().parse::<i32>().unwrap() == 1{
        entry_tick_value = response["proposal_open_contract"]["entry_tick_display_value"].to_string();
        exit_tick_value  = response["proposal_open_contract"]["exit_tick_display_value"].to_string();
        entry_tick_time  = format_date(response["proposal_open_contract"]["entry_tick_time"].to_string().parse::<u64>().unwrap());
        exit_tick_time   = format_date(response["proposal_open_contract"]["exit_tick_time"].to_string().parse::<u64>().unwrap());
        buy_price        = response["proposal_open_contract"]["buy_price"].to_string();
        profit           = response["proposal_open_contract"]["profit"].to_string();

        track_profit = track_profit + profit.parse::<f32>().unwrap();
        martingale = buy_price.parse::<i32>().unwrap();

        if profit.parse::<f32>().unwrap() > 0.0{
          profit = profit.green().to_string();
        }
        else{
          profit = profit.red().to_string();
        }

        data_table.push(Contract{
          contract_id: contract_id.to_owned(),
          contract_type: "DIGITEVEN".to_string(),
          entry_price: entry_tick_value.replace("\"", ""),
          exit_price: exit_tick_value.replace("\"", ""),
          entry_time: entry_tick_time,
          exit_time: exit_tick_time,
          amount: buy_price,
          profit: profit
        });
        render_table(&data_table);

        // Print Profit so far
        if track_profit > 0.0{
          println!("Profit {}",format!("{:.2}", track_profit).to_string().green());
        }
        else{
          println!("Profit {}",format!("{:.2}", track_profit).to_string().red());
        }
        // Print Current balance
        println!("Balance {}",balance);

        if response["proposal_open_contract"]["status"] == "lost"{
          // Apply Martingale
          martingale = martingale * 2;
        }
        if response["proposal_open_contract"]["status"] == "won"{
          // Reset Martingale
          martingale = trade_amount;
        }

        // Buys Contract
        contract = serde_json::json!({
          "buy": 1,
          "subscribe": 1,
          "price": martingale,
          "parameters": { 
            "amount": martingale, 
            "basis": "stake", 
            "contract_type": "DIGITEVEN", 
            "currency": "USD", 
            "duration": 1, 
            "duration_unit": "t", 
            "symbol": "R_100" }
        });
        socket.write_message(Message::Text(contract.to_string())).unwrap();
      }
    }
  }

}
