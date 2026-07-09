import express from 'express';
import { authMiddleware } from '../auth/middleware.js';

const app = express();
const router = express.Router();

router.post('/api/auth/login', authMiddleware);
router.get('/api/users', authMiddleware);
app.get('/health', (_req, res) => res.status(200).send('ok'));

export { app, router };