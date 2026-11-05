-- SQL: dijalankan SQLite di basis data sementara (hilang sesudah selesai).
CREATE TABLE buah (nama TEXT, harga INTEGER);
INSERT INTO buah VALUES ('mangga', 12000), ('jeruk', 8000), ('apel', 15000);
SELECT nama, harga FROM buah ORDER BY harga;
SELECT 'rata-rata', avg(harga) FROM buah;
