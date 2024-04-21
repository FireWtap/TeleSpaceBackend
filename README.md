
# Telespace 🚀

## Introduction
Telespace is a robust solution developed initially as a university project that has evolved to leverage Telegram as a platform for unlimited data storage. This tool allows users to store an extensive amount of data seamlessly by utilizing Telegram as the backend storage service.

## Table of Contents
- [Introduction](#introduction)
- [Features](#features)
- [Installation](#installation)
  - [Prerequisites](#prerequisites)
  - [Running the Project](#running-the-project)
  - [Compiling](#compiling)
- [Usage](#usage)
- [Project Structure](#project-structure)
  - [src](#src)
  - [entity](#entity)
  - [api](#api)
  - [service](#service)
- [Dependencies](#dependencies)
- [Configuration](#configuration)
- [Contributing](#contributing)
- [To-Do List](#to-do-list)
- [License](#license)

## Features
- **Unlimited File Storage**: Utilize the power of Telegram to store as much data as you need.
- **Custom Bot Setup**: Set up your own Telegram bot and specify the chat ID to start storing data.
- **Automated Scheduling**: Schedule uploads and downloads to manage data transfers effortlessly.

## Installation

### Prerequisites
- Ensure you have Rust installed on your machine. Visit [Rust's installation page](https://www.rust-lang.org/tools/install) for guidance.

### Running the Project
To run the project in debug mode, simply execute:
```bash
cargo run
```

### Compiling
To compile the project for production, use the following commands:
```bash
cargo build --release
```
Then, move the binary from the `target` directory to the root directory of the project and run it.

## Usage
To start using Telespace, configure your Telegram bot and specify the chat ID in the configuration file. Use the commands listed under [Running the Project](#running-the-project) and [Compiling](#compiling) to operate the system.

## Project Structure
### src
- `main.rs`: Main entry point of the application.

### entity
- Contains Rust models and entities for users, files, tasks, and data chunks. Managed via SeaOrm for database interactions.

### api
- Houses the API logic including JWT authentication, database connection pooling, and request handlers for tasks, files, and directories.

### service
- `task_queue.rs`: Manages task queuing and processing.
- `worker.rs`: Handles background worker tasks.

## Dependencies
- [Rocket](https://rocket.rs): For setting up the web server.
- [SeaOrm](https://www.sea-ql.org/SeaORM/): For ORM functionalities.

## Configuration
Configure the system by modifying the `config.toml` file located in the root directory. This file should include parameters such as your Telegram bot token and chat ID.

## Contributing
Contributions are welcome! For major changes, please open an issue first to discuss what you would like to change. Ensure to update tests as appropriate.

## To-Do List
- [ ] Optimize data handling and storage processes.
- [ ] Encryption/Decryption of everything that goes/comes to/from Telegram, preferably using Asymmetrical Encryption Methods.
- [ ] Resume uncomplete/failed task on start-up
- [ ] Multi Worker execution. (fairly tricky, given the raw dependencies we have)
- [ ] Unit Tests for every method
- [ ] General refactoring

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
