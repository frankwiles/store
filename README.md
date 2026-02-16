# Store CLI tool 

This small CLI tool makes is trivial for me to shove some data 
into one of my personal API projects in my cluster. 

## Usage 

First you should set the following environment variables: 

```shell
export STORE_API_TOKEN=example_token:XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
export STORE_PROJECT=sample-project 
export STORE_API_URL=https://example.com/api/storage/create/
```

These also also be passed on the command line but it's easier to 
use environment variables. 

You can send a JSON body as a string 

```shell
$ store '{"something": "1234", "enabled": False}' 
```

Or send `key=value` pairs which will get turned into a JSON 
object by the API

```shell
$ store key1=value1 key2=value2  
``` 

## Options

`--api-token` - Pass the API token directly rather than an env var 
`--api-url` - Override the default URL for testing or other purposes
`--project` - Pass the Project slug values directly
`--type` - Passes in the optional `data_type` extra argument for filtering purposes

