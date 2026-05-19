FROM node:20-alpine

WORKDIR /app

# Install deps first (layer cache)
COPY package.json ./
RUN npm install

# Copy source
COPY . .

EXPOSE 1420

CMD ["npm", "run", "dev"]
