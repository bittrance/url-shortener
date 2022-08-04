use actix_web::{dev::Service, get, web, App, HttpServer, Responder};
use anyhow::Error;
use azure_identity::AzureCliCredential;
use azure_svc_monitor::models::{
    AzureMetricsBaseData, AzureMetricsData, AzureMetricsDocument, AzureTimeSeriesData,
};
use azure_svc_monitor::operations::DEFAULT_ENDPOINT;
use azure_svc_monitor::ClientBuilder;
use chrono::Utc;
use reqwest::Client;
use std::sync::Arc;
use std::time::Instant;

async fn send_metrics(body: AzureMetricsDocument) -> Result<(), Error> {
    let endpoint = "https://westeurope.monitoring.azure.com";
    let id = "/subscriptions/78794a1a-58c3-4c66-8d09-95a82bc79416/resourceGroups/containerapp-example/providers/Microsoft.App/containerApps/containerapp-example";
    let url = format!("{}{}/metrics", endpoint, id);
    let client = Client::new();
    let res = client.post(url).json(&body).send().await?;
    Ok(())
}

fn make_body(metric: String, pt: AzureTimeSeriesData) -> AzureMetricsDocument {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let pt = AzureTimeSeriesData::new(1.0, 1.0, 1.0, 1);
    AzureMetricsDocument::new(
        now.to_string(),
        AzureMetricsData::new(AzureMetricsBaseData::new(
            metric,
            "Web".to_string(),
            vec![pt],
        )),
    )
}

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let credential = Arc::new(AzureCliCredential {});
    let client = ClientBuilder::new(credential)
        .endpoint(DEFAULT_ENDPOINT)
        .build();
    HttpServer::new(|| {
        App::new()
            .wrap_fn(|req, srv| {
                let fut = srv.call(req);
                async {
                    let ts = Instant::now();
                    let res = fut.await;
                    let latency = ts.elapsed().as_secs_f64();
                    let body = make_body(
                        "latency".to_string(),
                        AzureTimeSeriesData::new(latency, latency, latency, 1),
                    );
                    if let Err(e) = send_metrics(body).await {
                        eprintln!("{}", e);
                    }
                    res
                }
            })
            .service(greet)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

// curl -X POST https://login.microsoftonline.com/0a3a19f8-5fe4-4fb1-aabc-2bf971085ba2/oauth2/token -F "grant_type=client_credentials" -F "client_id=6636c256-ec3e-4d6e-b75f-9331c674c5c1" -F 'client_secret=VNe8Q~dS~8Bl_iIDUDPTLFgfgxHrrPf550AW7bxY' -F "resource=https://monitoring.azure.com/"
// curl -v -X POST https://westeurope.monitoring.azure.com/subscriptions/78794a1a-58c3-4c66-8d09-95a82bc79416/resourceGroups/containerapp-example/providers/Microsoft.App/containerApps/containerapp-example/metrics -H "Content-Type: application/json" -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiIsIng1dCI6ImpTMVhvMU9XRGpfNTJ2YndHTmd2UU8yVnpNYyIsImtpZCI6ImpTMVhvMU9XRGpfNTJ2YndHTmd2UU8yVnpNYyJ9.eyJhdWQiOiJodHRwczovL21vbml0b3JpbmcuYXp1cmUuY29tLyIsImlzcyI6Imh0dHBzOi8vc3RzLndpbmRvd3MubmV0LzBhM2ExOWY4LTVmZTQtNGZiMS1hYWJjLTJiZjk3MTA4NWJhMi8iLCJpYXQiOjE2NTM5NDE3MTUsIm5iZiI6MTY1Mzk0MTcxNSwiZXhwIjoxNjUzOTQ1NjE1LCJhaW8iOiJFMllBZ3VXNm43WThFSk9YVG4rdStaYlAvd01BIiwiYXBwaWQiOiI2NjM2YzI1Ni1lYzNlLTRkNmUtYjc1Zi05MzMxYzY3NGM1YzEiLCJhcHBpZGFjciI6IjEiLCJpZHAiOiJodHRwczovL3N0cy53aW5kb3dzLm5ldC8wYTNhMTlmOC01ZmU0LTRmYjEtYWFiYy0yYmY5NzEwODViYTIvIiwib2lkIjoiYjY1MzNjYWQtYjI2MC00MmVlLWJjOTAtZDc3ZDQzNzI4ZjgyIiwicmgiOiIwLkFUc0EtQms2Q3VSZnNVLXF2Q3Y1Y1FoYm9oMmVYd01BVHhsRXYxQ19MWWZyU0hnN0FBQS4iLCJzdWIiOiJiNjUzM2NhZC1iMjYwLTQyZWUtYmM5MC1kNzdkNDM3MjhmODIiLCJ0aWQiOiIwYTNhMTlmOC01ZmU0LTRmYjEtYWFiYy0yYmY5NzEwODViYTIiLCJ1dGkiOiJFY3NYUWh1dnRrNjRKdXotdlRsQ0FBIiwidmVyIjoiMS4wIn0.JyAZT4ZL1xjgOkum4w-e3VPK_yBAkI4-igXEx4bH-CaJ1Ba1-MOWiVxLSNRWY00hjrJbXVu7ze1pJkJtZiwazWOtK-vs0LilxkPsGZtHNDk58roLa2ewhl8dibxThks4x3IEiTwvU0zlQkScshWBXSnL0YA-UKuf6rdqwf6wsjg2SWyteWLKa1-2Pid-JLu-Yz_TinKQy5fp-XL59XDZIIBYOF7bcGDTUPGYRwCcEiIWM1LP4NSOhPkHLOYvcewb-O0kBnL1or5SDl-eUa7w3mH3JWV1uLyxvnKcWMXSSTBfGLS5RoSmMG0IEnoETA_IXgzosGV6MYCKtxiSkJqlWA" -d @file.json
// {
//     "time": "2022-05-30T20:27:00Z",
//     "data": {
//         "baseData": {
//             "metric": "Latency",
//             "namespace": "Web",
//             "dimNames": [
//                 "endpoint"
//             ],
//             "series": [
//                 {
//                     "dimValues": [
//                         "resolver"
//                     ],
//                     "min": 3,
//                     "max": 20,
//                     "sum": 28,
//                     "count": 3
//                 }
//             ]
//         }
//     }
// }
