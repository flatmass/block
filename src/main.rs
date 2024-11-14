use blockp_core::helpers::{self, fabric::NodeBuilder};
use blockp_configuration as configuration;

fn main() {
    helpers::init_logger().unwrap();

    let node = NodeBuilder::new()
        .with_service(Box::new(configuration::ServiceFactory))
        //.with_service(Box::new(time::ServiceFactory))
        .with_service(Box::new(fips::ServiceFactory));
    node.run();
}
