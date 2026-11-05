// JavaScript: dijalankan QuickJS di HP, atau node di laptop.
const kuadrat = [1, 2, 3, 4, 5].map((x) => x * x);
console.log("kuadrat:", kuadrat.join(", "));
console.log("jumlah:", kuadrat.reduce((a, b) => a + b, 0));
