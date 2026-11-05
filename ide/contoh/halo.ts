// TypeScript: jalan sesudah "TypeScript" dipasang lewat Lainnya > Bahasa.
interface Barang { nama: string; harga: number }

const keranjang: Barang[] = [
  { nama: "buku", harga: 45000 },
  { nama: "pena", harga: 7500 },
];
const total = keranjang.reduce((jumlah: number, b: Barang) => jumlah + b.harga, 0);
console.log(`${keranjang.length} barang, total ${total}`);
