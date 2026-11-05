% Prolog dijalankan sesudah paketnya dipasang (Lainnya > Bahasa).
:- use_module(library(lists)).

ortu(budi, sari).
ortu(sari, tono).
kakek(X, Z) :- ortu(X, Y), ortu(Y, Z).

main :-
    kakek(X, Z), write(X), write(' kakek dari '), write(Z), nl,
    length([a, b, c], N), write(N), write(' unsur'), nl.

:- initialization(main).
