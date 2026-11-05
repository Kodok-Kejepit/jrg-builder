-- Lua: jalan kalau penjalannya ikut di aplikasi (lihat Lainnya > Bahasa).
local function faktorial(n)
  if n <= 1 then return 1 end
  return n * faktorial(n - 1)
end

for i = 1, 5 do
  print(i .. "! = " .. faktorial(i))
end
