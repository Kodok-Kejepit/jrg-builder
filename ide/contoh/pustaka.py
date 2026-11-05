# Pustaka yang belum ada dipasang sendiri saat diimpor (butuh internet sekali, lalu tersimpan).
import numpy as np

a = np.arange(1, 7).reshape(2, 3)
print(a)
print("jumlah tiap kolom:", a.sum(axis=0))
print("rata-rata:", a.mean())
