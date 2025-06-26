Work Time TrackerA simple command-line utility written in Rust to track work time.DescriptionThis utility allows you to track time periods by recording start and stop times. It is designed to be simple and robust, ensuring that only one time period can be actively tracked at any given moment. The data is stored in a local file to persist between commands.FeaturesStart Tracking: Begin a new work period.Stop Tracking: End the current work period.Idempotent Operations: Running start or stop multiple times in a row will not create duplicate entries or errors.Data Persistence: Time entries are stored in a timesheet.json file in your home directory.Status Display: Shows the current tracking status and calculates the duration of the last session.UsagePrerequisitesYou need to have Rust and Cargo installed on your system. If you don't have them, you can install them from rustup.rs.Building the ProjectClone this repository or download the source code.Navigate to the project directory.Build the project using Cargo:cargo build --release
The executable will be located at target/release/work_time_tracker. You can add this to your system's PATH for easier access.CommandsThere are two main commands:1. Start a new tracking periodwork_time_tracker start
This command will:Check if a period is already being tracked.If not, it records the current timestamp as the start time.If a period is already active, it will inform you and do nothing.2. Stop the current tracking periodwork_time_tracker stop
This command will:Check if a period is currently being tracked.If yes, it records the current timestamp as the stop time and calculates the duration.If no period is active, it will inform you and do nothing.Data StorageThe application stores its data in a JSON file named .work_time_tracker.json located in your user's home directory (e.g., /home/user/.work_time_tracker.json).The JSON file has the following structure:{
  "periods": [
    {
      "start": "2023-10-27T10:00:00Z",
      "end": "2023-10-27T18:00:00Z"
    }
  ],
  "active_period_start": "2023-10-27T19:00:00Z"
}
periods: An array of completed time periods.active_period_start: The start time of the currently active period. If no period is active, this will be null.
