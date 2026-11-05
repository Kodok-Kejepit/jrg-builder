# Ruby dijalankan sesudah paketnya dipasang (Lainnya > Bahasa).
def faktorial(n)
  n <= 1 ? 1 : n * faktorial(n - 1)
end

nama = (gets || "dunia").strip
nama = "dunia" if nama.empty?
puts "Halo, #{nama}!"
puts "5! = #{faktorial(5)}"
puts [3, 1, 2].sort.map { |x| x * 10 }.inspect
