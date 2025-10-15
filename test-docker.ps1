# PowerShell test script for Docker deployment
# This script builds and tests the Docker container locally on Windows

$ErrorActionPreference = "Stop"

Write-Host "🔨 Building Docker image..." -ForegroundColor Cyan
docker build -t rust-research-mcp:test .

Write-Host ""
Write-Host "✅ Build successful!" -ForegroundColor Green
Write-Host ""

Write-Host "🚀 Starting container..." -ForegroundColor Cyan
$CONTAINER_ID = docker run -d `
  -p 3000:3000 `
  -e DOWNLOAD_DIR=/data `
  -e LOG_LEVEL=debug `
  -v "${PWD}/test-downloads:/data" `
  rust-research-mcp:test

Write-Host "Container ID: $CONTAINER_ID" -ForegroundColor Yellow
Write-Host ""

# Wait for server to start
Write-Host "⏳ Waiting for server to start..." -ForegroundColor Cyan
Start-Sleep -Seconds 5

Write-Host ""
Write-Host "🧪 Testing health endpoint..." -ForegroundColor Cyan
$healthResponse = Invoke-RestMethod -Uri "http://localhost:3000/mcp" -Method Get
$healthResponse | ConvertTo-Json

Write-Host ""
Write-Host "🧪 Testing MCP initialize..." -ForegroundColor Cyan
$initBody = @{
    jsonrpc = "2.0"
    id = 1
    method = "initialize"
    params = @{
        protocolVersion = "2024-11-05"
        capabilities = @{}
        clientInfo = @{
            name = "test"
            version = "1.0.0"
        }
    }
} | ConvertTo-Json -Depth 10

$initResponse = Invoke-RestMethod -Uri "http://localhost:3000/mcp" -Method Post -Body $initBody -ContentType "application/json"
$initResponse | ConvertTo-Json -Depth 10

Write-Host ""
Write-Host "🧪 Testing tools/list..." -ForegroundColor Cyan
$toolsBody = @{
    jsonrpc = "2.0"
    id = 2
    method = "tools/list"
    params = @{}
} | ConvertTo-Json

$toolsResponse = Invoke-RestMethod -Uri "http://localhost:3000/mcp" -Method Post -Body $toolsBody -ContentType "application/json"
$toolsResponse | ConvertTo-Json -Depth 10

Write-Host ""
Write-Host "📊 Container logs:" -ForegroundColor Cyan
Write-Host "====================" -ForegroundColor Cyan
docker logs $CONTAINER_ID | Select-Object -Last 20

Write-Host ""
Write-Host "🛑 Stopping container..." -ForegroundColor Cyan
docker stop $CONTAINER_ID | Out-Null
docker rm $CONTAINER_ID | Out-Null

Write-Host ""
Write-Host "✅ All tests completed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "📦 Image size:" -ForegroundColor Cyan
docker images rust-research-mcp:test --format "{{.Size}}"

Write-Host ""
Write-Host "🎉 Ready for Smithery deployment!" -ForegroundColor Green

