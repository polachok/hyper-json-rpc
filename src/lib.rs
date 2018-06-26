extern crate hyper;
extern crate futures;
#[macro_use]
extern crate serde;
extern crate serde_json;

use hyper::Server as HyperServer;
use hyper::Body;

macro_rules! rpc {
    ($(rpc $name:ident($arg:ident) -> $ret:ident;)*) => {
        use ::futures::future::Future;
        use ::futures::Stream;
        use ::futures::future::ok;
        use ::serde_json;
        use ::hyper::{StatusCode, Body};
        use ::std::sync::Arc;

        pub trait Service: Sized {
            $(
                fn $name(&self, req: $arg) -> Box<Future<Item = $ret, Error = ()> + Send>;
            )*
        }

        struct Serve<S> {
            inner: Arc<S>,
        }

        impl<S> Serve<S> {
            fn new(service: S) -> Self {
                Serve { inner: Arc::new(service) }
            }
        }

        impl<S> ::hyper::service::Service for Serve<S>
            where S: Service + Send + Sync + 'static
         {
            type ReqBody = Body;
            type ResBody = Body;
            type Error = Box<::std::error::Error + 'static + Send + Sync>;
            type Future = Box<Future<Item = ::hyper::Response<Self::ResBody>, Error = Self::Error> + Send>;

            fn call(&mut self, req: ::hyper::Request<Self::ReqBody>) -> Self::Future {
                let inner = Arc::clone(&self.inner);

                $(
                    if req.uri() == concat!("/", stringify!($name)) {
                        let resp = req.into_body().concat2().map_err(move |e| unimplemented!()).and_then(|body| {
                            serde_json::from_slice(&body).map_err(move |e| unimplemented!())
                        }).and_then(move |value| {
                            inner.$name(value).map_err(|e| unimplemented!())
                        }).then(move |result| match result {
                            Ok(value) => {
                                let data = serde_json::to_vec(&value).unwrap();
                                let mut response = ::hyper::Response::builder()
                                            .body(Body::from(data))
                                            .unwrap();
                                Ok(response)
                            }
                            _ => unimplemented!(),
                        });
                        return Box::new(resp);
                    }
                )*
                let mut resp = ::hyper::Response::builder()
                                    .status(StatusCode::NOT_FOUND)
                                    .body(Body::empty()).unwrap();
                Box::new(ok(resp))
            }
        }

    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        rpc! {
            rpc hello(String) -> String;
            rpc fuck(i32) -> i32;
        }

        struct S;

        impl Service for S {
            fn hello(&self, req: String) -> Box<Future<Item = String, Error = ()> + Send> {
                Box::new(ok(format!("Hello {}", req)))
            }

            fn fuck(&self, req: i32) -> Box<Future<Item = i32, Error = ()> + Send> {
                Box::new(ok(req))
            }
        }

        let new_service = move || {
            Ok::<_, ::std::io::Error>(Serve::new(S))
        };
        let addr = ([127,0,0,1], 3000).into();
        let server = ::hyper::Server::bind(&addr)
            .serve(new_service);
        ::hyper::rt::run(server.map_err(|e| panic!(e)))
    }
}
