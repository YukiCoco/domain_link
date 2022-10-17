use std::cell::RefCell;
use std::collections::HashMap;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Duration;
use std::{fs, thread};

use cloudflare::endpoints::dns::{self, ListDnsRecordsParams, UpdateDnsRecordParams};
use cloudflare::framework::apiclient::ApiClient;
use cloudflare::framework::*;
// use cloudflare::framework::HttpApiClient;
// use cloudflare::framework::HttpApiClientConfig;
// use cloudflare::framework::auth;
use trust_dns_resolver::config::*;
use trust_dns_resolver::Resolver;

use serde::{Deserialize, Serialize};
use serde_yaml::{self, Value};

#[derive(Deserialize, Serialize, Debug)]
struct Domain {
    ip: String,
    name: String,
    origin_name: String,
    #[serde(default)]
    id: String,
}

fn main() {
    let config = fs::read_to_string("./config.yml").unwrap();
    let mut config: Value = serde_yaml::from_str(config.as_str()).unwrap();
    let config = config.as_mapping_mut().unwrap();
    //let token = config.get("token").unwrap();
    let resolver =
        Resolver::new(ResolverConfig::cloudflare_tls(), ResolverOpts::default()).unwrap();

    // let token = auth::Credentials::UserAuthToken {
    //     token: String::from(token.as_str().unwrap()),
    // };

    let mut api_key = String::from("");
    let mut account_email = String::from("");
    let mut zone_identifier = String::from("");
    let mut sleep_duration: u64 = 0;

    {
        api_key = config.get("api_key").unwrap().as_str().unwrap().to_string();
        account_email = config
            .get("account_email")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        zone_identifier = config
            .get("zone_identifier")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        sleep_duration = config.get("sleep_duration").unwrap().as_u64().unwrap();
    }

    let token = auth::Credentials::UserAuthKey {
        email: account_email,
        key: api_key,
    };
    let client = HttpApiClient::new(
        token,
        HttpApiClientConfig::default(),
        Environment::Production,
    )
    .unwrap();

    let list_dns = dns::ListDnsRecords {
        zone_identifier: zone_identifier.as_str(),
        params: ListDnsRecordsParams::default(),
    };
    loop {
        let domains = config.get_mut("domains").unwrap();
        let domains = domains.as_sequence_mut().unwrap();
        let dnslist = client.request(&list_dns).unwrap();
        let cf_dns: Rc<RefCell<HashMap<String, String>>> =
            Rc::new(RefCell::new(HashMap::<String, String>::new()));
        for i in dnslist.result {
            //println!("{}, {}", i.name, i.id);
            cf_dns.borrow_mut().insert(i.name, i.id);
        }
        for domain in domains {
            let mut item: Domain = serde_yaml::from_value(domain.clone()).unwrap();

            // 后面加个 "."
            let response = resolver.lookup_ip(item.origin_name.clone() + ".").unwrap();

            // There can be many addresses associated with the name,
            //  this can return IPv4 and/or IPv6 addresses
            let address = response.iter().next().expect("no addresses returned!");

            if cf_dns.borrow().contains_key::<String>(&item.name) {
                item.id = cf_dns.borrow().get(&item.name).unwrap().clone();
            } else {
                // 不存在于 CloudFlare 的 DNS 列表
                if let IpAddr::V4(v4) = address {
                    let create_dns_params = dns::CreateDnsRecordParams {
                        ttl: Some(60),
                        proxied: Some(false),
                        name: item.name.as_str(),
                        content: dns::DnsContent::A { content: v4 },
                        priority: Some(0),
                    };
                    let create_dns = dns::CreateDnsRecord {
                        zone_identifier: zone_identifier.as_str(),
                        params: create_dns_params,
                    };
                    match client.request(&create_dns) {
                        Ok(_) => {
                            println!(
                                "Create dns record for {} with A record {}",
                                item.name.as_str(),
                                v4
                            );
                            continue;
                        }
                        Err(e) => {
                            println!("{}", e);
                            continue;
                        }
                    }
                }
            }
            //println!("{:?}", item);

            if address.is_ipv4() {
                let latest_ip = address.to_string();
                println!("Get an IP {} from domain {}", latest_ip, item.origin_name);
                if latest_ip != item.ip {
                    //Update dns to CloudFlare
                    if let IpAddr::V4(v4) = address {
                        let update_dns_params = UpdateDnsRecordParams {
                            ttl: Some(60),
                            proxied: Some(false),
                            name: item.name.as_str(),
                            content: dns::DnsContent::A { content: v4 },
                        };
                        let update_dns = dns::UpdateDnsRecord {
                            zone_identifier: zone_identifier.as_str(),
                            identifier: &item.id,
                            params: update_dns_params,
                        };
                        match client.request(&update_dns) {
                            Ok(_) => (),
                            Err(e) => {
                                println!("{}", e);
                                continue;
                            }
                        }
                        println!("update ip: {}, name: {}", latest_ip, item.name);

                        //Save to configuration
                        //let domains = config.get_mut("domains").unwrap();
                        match domain.get_mut("ip") {
                            Some(ip) => {
                                *ip = Value::String(latest_ip);
                            }
                            None => (),
                        }
                    }
                } else {
                    println!("Nothing to update");
                }
            }
        }
        fs::write(
            "./config.yml",
            serde_yaml::to_string(&config.clone()).unwrap(),
        )
        .unwrap();
        thread::sleep(Duration::from_secs(sleep_duration));
    }
}
