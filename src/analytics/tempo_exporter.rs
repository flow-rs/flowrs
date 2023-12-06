use std::fmt::{Debug, Formatter};
use opentelemetry::Key;
use opentelemetry_sdk::export::trace::{ExportResult, SpanData, SpanExporter};

pub struct TempoExporter {
    client: reqwest::blocking::Client,
    tempo_host: String,
}

impl Default for TempoExporter {
    fn default() -> Self {
        TempoExporter {
            client: reqwest::blocking::Client::new(),
            tempo_host: "http://localhost:4318/v1/traces".to_string(),
        }
    }
}

impl TempoExporter {
    pub fn new(tempo_host: String) -> Self {
        TempoExporter {
            client: reqwest::blocking::Client::new(),
            tempo_host,
        }
    }
}

impl Debug for TempoExporter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TempoExporter")
    }
}

impl SpanExporter for TempoExporter {
    fn export(&mut self, batch: Vec<SpanData>) -> futures_core::future::BoxFuture<'static, ExportResult> {
        println!("pushing to tempo");
        // we are only interested in flowrs code namespace traces for now
        let filtered: Vec<_> = batch.into_iter().filter(|span| {
            let code_namespace = span.attributes.iter().find(|attr| {
                attr.key == Key::new("code.namespace")
            }).map(|attr| {
                attr.value.as_str()
            });
            match code_namespace {
                Some(code) => {
                    let valid = code.starts_with("flowrs");
                    if !valid {
                        println!("filtered out {code}");
                    }
                    valid
                }
                None => false
            }
        }).collect();

        let span_data = opentelemetry_stdout::SpanData::from(filtered);

        let response = self.client.post(&self.tempo_host).json(&span_data).send();
        match response {
            Ok(resp) => {
                if resp.status() == 200 {
                    // alright
                } else {
                    println!("Unsuccessful push: {:?}", resp.text());
                    println!("Span data: {:?}", span_data);
                }
            }
            Err(e) => {
                println!("Fatal Error when pushing: {e:?}");
            }
        }

        Box::pin(std::future::ready(Ok(())))
    }

    fn shutdown(&mut self) {
        // do nothing
    }

    fn force_flush(&mut self) -> futures_core::future::BoxFuture<'static, ExportResult> {
        // do nothing
        Box::pin(std::future::ready(Ok(())))
    }
}

