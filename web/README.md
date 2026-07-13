# Code Terroir API Testing GUI

This is a web-based interface to test and explore the Code Terroir API endpoints.

## Quick Start

### 1. Start the API server (in one terminal):
```bash
cd /path/to/code_terroir_warp
RUST_LOG=info cargo run
```
The API server will start on `http://localhost:3030`

### 2. Start the GUI server (in another terminal):
```bash
cd /path/to/code_terroir_warp
python3 serve_gui.py
```
The GUI will be available at `http://localhost:8080`

### 3. Open your browser and navigate to:
```
http://localhost:8080
```

## Features

- **Health Check**: Test if the API server is running
- **Authentication**: Test login, register, and token refresh endpoints
- **Products**: Test product CRUD operations
- **Batches**: Test batch management endpoints
- **QR Codes**: Test QR code generation and retrieval
- **Public Pages**: Test public traceability pages
- **Request Customization**: Add custom headers and request bodies
- **Response Display**: View detailed responses with headers, status codes, and execution times

## Available Endpoints

### Health Check
- `GET /health` - Check server status

### Authentication
- `POST /api/v1/auth/login` - User login
- `POST /api/v1/auth/register` - User registration
- `POST /api/v1/auth/refresh` - Refresh authentication token

### Products
- `GET /api/v1/products` - List all products
- `POST /api/v1/products` - Create a new product
- `GET /api/v1/products/{id}` - Get specific product

### Batches
- `GET /api/v1/batches` - List all batches
- `POST /api/v1/batches` - Create a new batch
- `GET /api/v1/batches/{id}` - Get specific batch

### QR Codes
- `POST /api/v1/qr/generate/{batch_id}` - Generate QR code for batch
- `GET /api/v1/qr/{qr_id}` - Get QR code details

### Public Pages
- `GET /t/{trace_id}` - Public traceability page

## Tips

- Use the "Request Settings" section to customize headers and request bodies
- All responses show HTTP status, headers, and execution time
- The server status indicator shows if the API is reachable
- You can test custom trace IDs in the Public Pages section

## Troubleshooting

### CORS Issues
The GUI server includes CORS headers, but if you encounter issues:
- Make sure both servers (API and GUI) are running
- Check that the Base URL in settings matches your API server address

### Connection Refused
- Verify the API server is running on port 3030
- Check if any firewall is blocking the connections
- Try accessing http://localhost:3030/health directly

### Port Already in Use
If port 8080 is already in use, modify the `PORT` variable in `serve_gui.py`