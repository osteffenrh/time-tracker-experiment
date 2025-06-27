# Work Time Tracker

A simple command-line utility written in Rust to track work time.

## Description

This utility allows you to track time periods by recording start and stop times. It is designed to be simple and robust, ensuring that only one time period can be actively tracked at any given moment. The data is stored in a local file to persist between commands. You can also generate reports for tracked time for the current day, week, or month.

## Features

* **Start Tracking**: Begin a new work period.
* **Stop Tracking**: End the current work period.
* **Idempotent Operations**: Running `start` or `stop` multiple times in a row will not create duplicate entries or errors.
* **Data Persistence**: Time entries are stored in a `.work_time_tracker.json` file in your home directory.
* **Time Reporting**: Get summaries of tracked time for the current day, week, or month.

## Usage

### Prerequisites

You need to have Rust and Cargo installed on your system. If you don't have them, you can install them from [rustup.rs](https://rustup.rs/).

### Building the Project

1. Clone this repository or download the source code.
2. Navigate to the project directory.
3. Build the project using Cargo:
   ```bash
   cargo build --release
   ```
4. The executable will be located at `target/release/work_time_tracker`. You can add this to your system's PATH for easier access.

### Commands

#### 1. Start a new tracking period

```bash
work_time_tracker start
```

This command will:
* Check if a period is already being tracked.
* If not, it records the current timestamp as the start time.
* If a period is already active, it will inform you and do nothing.

#### 2. Stop the current tracking period

```bash
work_time_tracker stop
```

This command will:
* Check if a period is currently being tracked.
* If yes, it records the current timestamp as the stop time and calculates the duration.
* If no period is active, it will inform you and do nothing.

#### 3. Report Tracked Time

You can get a summary of tracked time over different periods. These commands are read-only and will not modify your data file. They include any currently active session in the calculation.

-   **For the current day:**
    ```bash
    work_time_tracker today
    ```

-   **For the current week (Monday to Sunday):**
    ```bash
    work_time_tracker week
    ```

-   **For the current month:**
    ```bash
    work_time_tracker month
    ```

## Data Storage

The application stores its data in a JSON file named `.work_time_tracker.json` located in your user's home directory (e.g., `/home/user/.work_time_tracker.json`).

The JSON file has the following structure:
```json
{
  "periods": [
    {
      "start": "2023-10-27T10:00:00Z",
      "end": "2023-10-27T18:00:00Z"
    }
  ],
  "active_period_start": "2023-10-27T19:00:00Z"
}
```

- `periods`: An array of completed time periods.
- `active_period_start`: The start time of the currently active period. If no period is active, this will be `null`.

