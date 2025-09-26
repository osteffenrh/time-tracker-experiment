# Work Time Tracker

A simple command-line utility written in Rust to track work time.

## Description

This utility allows you to track time periods by recording start and stop times. It is designed to be simple and robust, ensuring that only one time period can be actively tracked at any given moment. The data is stored in a local file to persist between commands. You can also generate reports for tracked time for the current day, week, or month.

## Features

*   **Start Tracking**: Begin a new work period.
*   **Stop Tracking**: End the current work period.
*   **Time Specification**: Start or stop tracking at a specific time using offsets (e.g., `-1:00`) or absolute times (e.g., `14:30`).
*   **Sick Days**: Mark specific dates as sick days.
*   **Idempotent Operations**: Running `start` or `stop` multiple times in a row will not create duplicate entries or errors.
*   **Data Persistence**: Time entries are stored in a local JSON file.
*   **XDG Compliant**: Follows the XDG Base Directory Specification for data and configuration files.
*   **Time Reporting**: Get summaries of tracked time for the current day, week, or month.
*   **Detailed Reports**: View a daily breakdown of the current week's tracked time and a progress bar for the current day.
*   **Easy Editing**: Open the data file in your default editor with the `edit` command.

## Usage

### Prerequisites

You need to have Rust and Cargo installed on your system. If you don't have them, you can install them from [rustup.rs](https://rustup.rs/).

### Building the Project

1.  Clone this repository or download the source code.
2.  Navigate to the project directory.
3.  Build the project using Cargo:
    ```bash
    cargo build --release
    ```
4.  The executable will be located at `target/release/time_tracker`. You can add this to your system's PATH for easier access.

### Commands

#### 1. Start a new tracking period

```bash
time_tracker start [time]
```

This command will:

*   Check if a period is already being tracked.
*   If not, it records the current timestamp as the start time.
*   If a period is already active, it will inform you and do nothing.
*   The optional `[time]` argument can be an offset (e.g., `-1:00`, `+0:30`) or an absolute time (e.g., `14:30`).

#### 2. Stop the current tracking period

```bash
time_tracker stop [time]
```

This command will:

*   Check if a period is currently being tracked.
*   If yes, it records the current timestamp as the stop time and calculates the duration.
*   If no period is active, it will inform you and do nothing.
*   The optional `[time]` argument can be an offset (e.g., `-1:00`, `+0:30`) or an absolute time (e.g., `14:30`).

#### 3. Mark a sick day

```bash
time_tracker sick [date]
```

This command marks a specific date as a sick day. The `[date]` argument should be in `YYYY-MM-DD` format. If omitted, it defaults to the current day.

#### 4. Edit the data file

```bash
time_tracker edit
```

This command opens the `timesheet.json` data file in your default editor (as specified by the `$EDITOR` environment variable).

#### 5. Report Tracked Time

You can get a summary of tracked time over different periods. These commands are read-only and will not modify your data file. They include any currently active session in the calculation.

*   **For the current day:**
    ```bash
    time_tracker today
    ```

*   **For the current week (Monday to Sunday):**
    ```bash
    time_tracker week
    ```

*   **For the current month:**
    ```bash
    time_tracker month
    ```

*   **Daily breakdown for the current week:**
    ```bash
    time_tracker week-breakdown
    ```

*   **Progress bar for the current day:**
    ```bash
    time_tracker progress
    ```

## Configuration

The application can be configured via a TOML file located at `~/.config/time_tracker/config.toml`.

### Options

*   `work_hours`: Customize the number of work hours for each day of the week.

### Example Configuration

```toml
[work_hours]
monday = 7.5
tuesday = 7.5
wednesday = 7.5
thursday = 7.5
friday = 7.0
saturday = 0.0
sunday = 0.0
```

If the configuration file is not present or is empty, the application uses default values (8 hours for weekdays, 0 for weekends).

## Data Storage

The application stores its data in a JSON file named `timesheet.json` located in `~/.local/share/time_tracker/`.

The application will automatically migrate the data file from the legacy location (`~/.work_time_tracker.json`) to the new XDG-compliant location.

The JSON file has the following structure:

```json
{
  "periods": [
    {
      "start": "2023-10-27T10:00:00Z",
      "end": "2023-10-27T18:00:00Z"
    }
  ],
  "active_period_start": "2023-10-27T19:00:00Z",
  "sick_days": [
    "2023-10-26"
  ]
}
```

*   `periods`: An array of completed time periods.
*   `active_period_start`: The start time of the currently active period. If no period is active, this will be `null`.
*   `sick_days`: An array of dates marked as sick days.