[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$browser = New-Object System.Net.WebClient

$browser.Proxy.Credentials =[System.Net.CredentialCache]::DefaultNetworkCredentials;

$url=$args[0]

$dest=$args[1]

write-output "Download file [$url] to [$dest]"

Invoke-WebRequest $url -OutFile $dest
