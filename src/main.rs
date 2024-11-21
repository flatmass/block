use blockp_configuration as configuration;
use blockp_core::helpers::fabric::NodeBuilder;

fn main() {
    let _logger_guard = log_custom::init_logger().unwrap();

    let node = NodeBuilder::new()
        .with_service(Box::new(configuration::ServiceFactory))
        //.with_service(Box::new(time::ServiceFactory))
        .with_service(Box::new(fips::ServiceFactory));
    node.run();
}
