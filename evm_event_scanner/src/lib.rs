use async_trait::async_trait;
use serde_json::Value;
use reqwest;
use poller_core::Base64Bytes;
use std::sync::Arc;

#[async_trait]
pub trait EventScannerAdapter<MovedData: 'static> where MovedData: Send + Clone + Sync {
    async fn get_move_data(&self) -> Arc<MovedData>;
    async fn log_string(&self, number: u128) -> String;
    async fn height_string(&self, number: u128) -> String;

    async fn proc_portion(
            my_data: Arc<MovedData>,
            data: Value,
            thread_num: &str,
            heigth: &str,
    ) -> Base64Bytes;

    async fn helt_for_block(
        &self,
        number: u128,
    ) {
        use tokio::time::{sleep, Duration};
        loop {
            sleep(Duration::from_millis(500)).await;
            let request_string = self.height_string(number).await;
            println!("query string polling for new block: {}",request_string);
            let data = match self.request(request_string).await {
                Ok(d) => d,
                Err(_) => continue,
            };
            println!("received data while fetching block size:\n{}",data);
            if data["status"] != "1".to_string() { 
                println!("ERROR| requesting height was not successful"); 
                continue;
            }
            let height = match data["result"].as_str() {
                Some(s) => s,
                None => continue,
            };
            let height = match height.parse::<u128>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            if height >= number+2 {
                break;
            }
        }
    }

    async fn request(
        &self, 
        request_string: String
    ) -> Result<Value,reqwest::Error> {
            reqwest::get(&request_string)
                .await?
                .json::<Value>()
                .await
    }

    async fn request_for_logs(
        &self,
        number: u128, 
    ) -> Vec<Value> {
        loop {
            let request_string = self.log_string(number).await;
            let data = match self.request(request_string).await {
                Ok(d) => d,
                Err(_) => continue,
            };

            if data["status"] == "0".to_string() && 
                data["message"] == "No records found".to_string() {
                return Vec::new();
            }
            if data["status"] != "1".to_string() { 
                println!("ERROR| request was not successful"); 
                continue;
            }
            match data["result"].as_array() {
                Some(s) => return s.clone(),
                None => continue,
            }
        }
    }
    async fn process_epoch(
        &self, 
        number: u128,
    ) -> Result<Vec<Base64Bytes>,Box<dyn std::error::Error>> {
        println!("starting helt");
        self.helt_for_block(number).await;
        let result = self.request_for_logs(number).await;
        println!("got data {:?}",result);
        let rows_count = result.len();
        let (agr_sender, agr_receiver) = std::sync::mpsc::channel::<String>();
        let my_data = self.get_move_data().await;
        for (i,data) in result.into_iter().enumerate() {

            let thread_num = i.to_string().clone();
            let s = agr_sender.clone();
            let heigth = number.to_string().clone();
            let md: Arc<MovedData> = my_data.clone();
            tokio::spawn(async move {
                let res: String = Self::proc_portion(md, data, &thread_num, &heigth).await;
                s.send(res).unwrap();
            });
        }
        let mut res = Vec::new();
        let mut get_count = 0;
        while get_count < rows_count {
            res.push(agr_receiver.recv()?);
            get_count += 1;
        }
        Ok(res)
    }
}

