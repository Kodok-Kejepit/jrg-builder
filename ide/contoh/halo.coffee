# CoffeeScript diubah jadi JavaScript sesudah paketnya dipasang (Lainnya > Bahasa).
kuadrat = (x) -> x * x
console.log "7 kuadrat = #{kuadrat 7}"
console.log (kuadrat n for n in [1..5]).join " "
