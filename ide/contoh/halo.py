# Python: jalan sesudah "Python" dipasang lewat Lainnya > Bahasa (atau python3 di laptop).
import math

try:
    nama = input()
except EOFError:
    nama = ""
print(f"Halo, {nama or 'dunia'}!")
print("akar 2 =", round(math.sqrt(2), 6))
print("kuadrat:", [x * x for x in range(1, 6)])
