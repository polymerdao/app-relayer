import winston from 'winston';

// Create a logger instance that logs only to the console
export const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || 'info',
  format: winston.format.combine(
    winston.format.timestamp({
      format: 'YYYY-MM-DD HH:mm:ss'
    }),
    winston.format.errors({ stack: true }),
    winston.format.splat(),
    winston.format.json()
  ),
  defaultMeta: { service: 'ts-relayer' },
  transports: [
    new winston.transports.Console({
      format: winston.format.combine(
        winston.format.colorize(),
        winston.format.printf(({ level, message, timestamp, ...metadata }) => {
          // Extract relevant metadata
          const meta = Object.keys(metadata).length > 0 
            ? JSON.stringify(metadata, null, 2)
            : '';
          
          // Format the log message
          return `${timestamp} [${level}]: ${message} ${meta}`;
        })
      )
    })
  ]
});
