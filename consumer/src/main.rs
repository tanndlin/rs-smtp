use amiquip::{Connection, ConsumerMessage, ConsumerOptions, QueueDeclareOptions};

fn main() {
    let mut connection = Connection::insecure_open("amqp://rabbitmq").unwrap();
    let channel = connection.open_channel(None).unwrap();
    let queue = channel
        .queue_declare(
            "mail",
            QueueDeclareOptions {
                durable: true,
                ..QueueDeclareOptions::default()
            },
        )
        .expect("Failed to declare a queue");

    // Start the consumer
    let consumer = queue
        .consume(ConsumerOptions::default())
        .expect("Failed to start consuming messages");

    println!("Waiting for messages. Press Ctrl-C to exit.");

    // Iterate over incoming messages
    for message in consumer.receiver().iter() {
        match message {
            ConsumerMessage::Delivery(delivery) => {
                let body = String::from_utf8_lossy(&delivery.body);
                println!("Received message: {}", body);

                // Acknowledge the message
                delivery
                    .ack(&channel)
                    .expect("Failed to acknowledge message");
            }
            other => {
                println!("Other consumer message: {:?}", other);
            }
        }
    }
}
