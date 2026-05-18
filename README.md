# Rust

### Test Applications with Rust


How to run:
Go to each application and type
>cargo run
>

### 1. mars_calc :: Calculation of Weight on Mars - Test command line application
### 2. Http Server :: A Simple HTTP Server in RUST - Simple version to serve HTML/CSS/JavaScript files

##### Environment variable: PUBLIC_PATH

Do not forget to set PUBLIC_PATH environment variable with the path of public folder

for example for MacOS:

>echo $PUBLIC_PATH
>
>export PUBLIC_PATH=/Users/antonis/dev/rust_test/http_server/target/release/http_server_0.0.1_mac/public
>
>echo $PUBLIC_PATH
>
>./http_server
>


### 3. Icecast 2 Mp3 Streamer :: Icecast2 client to stream mp3 files to your e-radio. 
##### Configuration:
folders_truck_e=C:\music\truck-e  -> loads from this folder 3 mp3 files then

folders_truck_1=C:\music\truck-1  -> Loads 1 mp3 from this folder

folders_truck_3=C:\music\trucks-3 -> loads 2 or 5 (randomly) mp3 files from this folder 

folders_options=C:\music\truck-options -> if there is any then loads 1 mp3 from this folder.
then it loops again by taking new mp3 files with the same way


