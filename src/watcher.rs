use std::sync::Arc;

use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct FielmannResponseItem {
    date: String,
    free: String,
}

#[derive(Deserialize, Debug)]
struct FielmannTimeslot {
    date: String,
    timeslots: Timeslot,
}

#[derive(Deserialize, Debug)]
struct Timeslot {
    from: String,
    to: String,
}

#[derive(Debug)]
pub struct Executor {
    client: Client,
    stores: Vec<String>,
    channel: poise::serenity_prelude::ChannelId,
    discord_client: Arc<poise::serenity_prelude::Http>,
}

impl Executor {
    pub fn new(
        channel: poise::serenity_prelude::ChannelId,
        discord_client: Arc<poise::serenity_prelude::Http>,
    ) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Accept",
            "application/json, text/plain, */*".parse().unwrap(),
        );
        headers.insert("Accept-Language", "en-US,en;q=0.5".parse().unwrap());
        headers.insert(
            "Accept-Encoding",
            "gzip, deflate, br, zstd".parse().unwrap(),
        );
        headers.insert("DNT", "1".parse().unwrap());
        headers.insert("Connection", "keep-alive".parse().unwrap());
        headers.insert(
            "Referer",
            "https://termine.fielmann.de/find-branch?service=CL_CF"
                .parse()
                .unwrap(),
        );
        headers.insert("Cookie", "OptanonConsent=isGpcEnabled=0&datestamp=Fri+Aug+02+2024+22%3A57%3A12+GMT%2B0200+(Central+European+Summer+Time)&version=202401.1.0&browserGpcFlag=1&isIABGlobal=false&hosts=&genVendors=&consentId=682341bf-8f7d-41fa-9d4e-ee1d7ffae4dd&interactionCount=1&landingPath=NotLandingPage&groups=C0001%3A1%2CC0002%3A1%2CC0004%3A1;".parse().unwrap());
        headers.insert("Sec-Fetch-Dest", "empty".parse().unwrap());
        headers.insert("Sec-Fetch-Mode", "cors".parse().unwrap());
        headers.insert("Sec-Fetch-Site", "same-origin".parse().unwrap());
        headers.insert("Sec-GPC", "1".parse().unwrap());
        headers.insert("TE", "trailers".parse().unwrap());

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            client,
            stores: vec![String::from("0885"), String::from("0103")],
            discord_client,
            channel,
        }
    }
    pub async fn start(&self, signal: std::sync::mpsc::Receiver<()>) {
        let mut start = tokio::time::Instant::now();
        loop {
            match signal.try_recv() {
                Ok(_) => {
                    dbg!("Bot received signal to stop");
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(e) => {
                    dbg!(e);
                    break;
                }
            }

            // if one minute has passed
            if start.elapsed().as_secs() >= 60 {
                let mut dates = std::collections::HashMap::new();
                let mut send_message = false;
                for store in &self.stores {
                    let times = self.do_request(store).await.unwrap();
                    if !times.is_empty() {
                        send_message = true;
                    }
                    dates.insert(store, times);
                }

                let mut message = String::new();
                for (store, times) in dates {
                    let store_name = match store.as_str() {
                        "0885" => "Bonn city center",
                        "0103" => "Bonn Kölnstraße",
                        _ => "Unknown",
                    };
                    message.push_str(&format!("Store: {}\n", store_name));
                    if times.is_empty() {
                        message.push_str("No dates available\n\n");
                        continue;
                    }
                    for time in times {
                        message.push_str(&format!(
                            "Date: {}\nFrom: {}\nTo: {}\n\n",
                            time.date, time.timeslots.from, time.timeslots.to
                        ));
                    }
                }

                if send_message {
                    let m = poise::serenity_prelude::CreateMessage::new().content(message);
                    self.channel
                        .send_message(&self.discord_client, m)
                        .await
                        .unwrap();
                }

                start = tokio::time::Instant::now();
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    async fn do_request(&self, store_code: &str) -> Result<Vec<FielmannTimeslot>, reqwest::Error> {
        let url = format!(
            "https://termine.fielmann.de/api/v3/times/001-{}/free/CL_CF/next",
            store_code
        );
        let dates = self
            .client
            .get(url)
            .send()
            .await
            .unwrap()
            .json::<Vec<FielmannTimeslot>>()
            .await
            .unwrap();
        Ok(dates)
    }
}
