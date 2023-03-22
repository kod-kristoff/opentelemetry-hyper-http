use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use opentelemetry::sdk::trace::Config;
use opentelemetry::sdk::{trace as sdktrace, Resource};
use opentelemetry::trace::TraceError;
use opentelemetry::{
    global,
    sdk::export::trace::stdout,
    sdk::{
        propagation::TraceContextPropagator,
        trace::{self, Sampler},
    },
    trace::Span,
};
use opentelemetry::{
    trace::{TraceContextExt, Tracer},
    Key, KeyValue,
};
use opentelemetry_contrib::trace::exporter::jaeger_json::{JaegerJsonExporter, JaegerJsonRuntime};
use opentelemetry_http::HeaderExtractor;
use std::error::Error;
use std::{convert::Infallible, net::SocketAddr};

async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let parent_cx = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(req.headers()))
    });
    let mut span = global::tracer("example/server").start_with_context("hello", &parent_cx);
    span.add_event("handling this...", Vec::new());

    Ok(Response::new("Hello, World!".into()))
}

fn init_tracer2() -> impl Tracer {
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Install stdout exporter pipeline to be able to retrieve the collected spans.
    // For the demonstration, use `Sampler::AlwaysOn` sampler to sample all traces. In a production
    // application, use `Sampler::ParentBased` or `Sampler::TraceIdRatioBased` with a desired ratio.
    stdout::new_pipeline()
        .with_trace_config(trace::config().with_sampler(Sampler::AlwaysOn))
        .install_simple()
}

fn init_tracer() -> sdktrace::Tracer {
    global::set_text_map_propagator(TraceContextPropagator::new());

    JaegerJsonExporter::new(
        "logs".into(),
        "otlp".into(),
        "example/server".into(),
        opentelemetry::runtime::Tokio,
    )
    .install_batch()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let _tracer = init_tracer();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on {addr}");
    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }

    global::shutdown_tracer_provider();
    Ok(())
}
