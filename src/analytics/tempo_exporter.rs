use std::fmt::{Debug, Formatter};
use opentelemetry::Key;
use opentelemetry_sdk::export::trace::{ExportResult, SpanData, SpanExporter};

pub struct TempoExporter {
    client: reqwest::blocking::Client,
}

impl Default for TempoExporter {
    fn default() -> Self {
        TempoExporter {
            client: reqwest::blocking::Client::new(),
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
        // filter out traces from the hyper namespace, to avoid infinite recursion when pushing traces
        let filtered: Vec<_> = batch.into_iter().filter(|span| {
            let code = span.attributes.get(&Key::from_static_str("code.namespace"));
            match code {
                Some(code) => {
                    !code.as_str().starts_with("hyper")
                }
                None => true
            }
        }).collect();

        let span_data = opentelemetry_stdout::SpanData::from(filtered);

        let response = self.client.post("http://localhost:4318/v1/traces").json(&span_data).send();
        println!("Response: {:?}", response);

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