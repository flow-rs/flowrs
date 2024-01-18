use std::fmt::{Debug, Formatter};
use opentelemetry::Key;
use opentelemetry_sdk::export::trace::{ExportResult, SpanData, SpanExporter};

pub struct OtlpExporter {
    client: reqwest::blocking::Client,
    otlp_host: String,
}

impl Default for OtlpExporter {
    fn default() -> Self {
        OtlpExporter {
            client: reqwest::blocking::Client::new(),
            otlp_host: "http://localhost:4318/v1/traces".to_string(),
        }
    }
}

impl OtlpExporter {
    pub fn new(tempo_host: String) -> Self {
        OtlpExporter {
            client: reqwest::blocking::Client::new(),
            otlp_host: tempo_host,
        }
    }
}

impl Debug for OtlpExporter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TempoExporter")
    }
}

impl SpanExporter for OtlpExporter {
    fn export(&mut self, batch: Vec<SpanData>) -> futures_core::future::BoxFuture<'static, ExportResult> {
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

        let response = self.client.post(&self.otlp_host).json(&span_data).send();
        match response {
            Ok(resp) => {
                if resp.status() == 200 {
                    // alright
                } else {
                    println!("Unsuccessful push: {:?}", resp.text());
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

