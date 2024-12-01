SELECT 'CREATE DATABASE kodingkorp'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'kodingkorp')\gexec