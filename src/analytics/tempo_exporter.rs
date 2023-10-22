use std::fmt::{Debug, Formatter};
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
        let span_data = opentelemetry_stdout::SpanData::from(batch);

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