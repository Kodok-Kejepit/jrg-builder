; Scheme dijalankan sesudah paketnya dipasang (Lainnya > Bahasa).
(define (faktorial n)
  (if (<= n 1) 1 (* n (faktorial (- n 1)))))

(display "5! = ")
(display (faktorial 5))
(newline)
(display (map (lambda (x) (* x x)) (list 1 2 3)))
(newline)
