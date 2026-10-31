#!/usr/bin/env python3
"""
Kalkulator cross-check INDEPENDEN (bukan port dari kode Rust) untuk nilai
acuan probabilitas kategori & matriks informasi item di
tests/fixtures/irt_probability_information.json (fixture Fase 0
theta-engine). MDISC/MDIFF tidak dihitung di sini.

Sumber formula (dipakai langsung dari literatur):
  - Dikotomus (M2PL/M3PL/M4PL): Reckase (2009), Multidimensional Item
    Response Theory, bab 4 (bentuk kompensatori multidimensional);
    upper asymptote `u` pada M4PL: Barton & Lord (1981), "An upper
    asymptote for the three-parameter logistic item-response model",
    ETS Research Report RR-81-20.
  - Graded response: Muraki & Carlson (1995); Samejima boundary-curve
    formulation.
  - Informasi Fisher multidimensional (MD): I(theta) = sum_k outer(grad
    P_k, grad P_k) / P_k -- formula umum "outer-product-of-gradients"
    utk exponential family.

Kegunaan: output skrip ini bisa dibandingkan dengan nilai acuan di fixture
utk verifikasi cepat tanpa menjalankan engine, atau bila fixture perlu
diperluas dan kasus barunya ingin dicek dulu secara manual.
"""
import json
import math
import sys


def logistic(z: float) -> float:
    # Logistik stabil, dipecah menurut tanda z supaya exp() selalu dipanggil
    # dengan argumen <= 0 (tanpa overflow, tanpa memotong nilai logit).
    if z >= 0.0:
        return 1.0 / (1.0 + math.exp(-z))
    e = math.exp(z)
    return e / (1.0 + e)


def logistic_derivative(z: float) -> float:
    # P*(1 - P*) = logistic(z) * logistic(-z); tidak memakai 1 - logistic(z)
    # karena pengurangan itu membulat ke 0 untuk z > ~36.
    return logistic(z) * logistic(-z)


def dich_probabilities(a, d, g, u, theta):
    z = d + sum(aj * tj for aj, tj in zip(a, theta))
    p1 = g + (u - g) * logistic(z)
    p0 = (1.0 - u) + (u - g) * logistic(-z)
    return [p0, p1]


def dich_information(a, d, g, u, theta):
    z = d + sum(aj * tj for aj, tj in zip(a, theta))
    p0, p1 = dich_probabilities(a, d, g, u, theta)
    factor = (u - g) * logistic_derivative(z)
    grad1 = [factor * aj for aj in a]
    n = len(a)
    info = [[0.0] * n for _ in range(n)]
    # Suku dengan probabilitas 0 (atau gradien persis 0) berkontribusi 0
    # (limit grad^2 / P -> 0 di ekor logistik), bukan pembagian dengan nol.
    for p in (p1, p0):
        if p <= PROB_FLOOR:
            continue
        for i in range(n):
            for j in range(n):
                outer = grad1[i] * grad1[j]
                if outer != 0.0:
                    info[i][j] += outer / p
    return info


# Lantai probabilitas kategori: bilangan float positif terkecil (normal).
# Hanya menjaga ln P tetap finite; bukan klem dua sisi.
PROB_FLOOR = sys.float_info.min


def graded_boundary_curves(a, d, theta):
    pstar, qstar, deriv = [1.0], [0.0], [0.0]
    for dk in d:
        z = dk + sum(aj * tj for aj, tj in zip(a, theta))
        pstar.append(logistic(z))
        qstar.append(logistic(-z))
        deriv.append(logistic_derivative(z))
    pstar.append(0.0)
    qstar.append(1.0)
    deriv.append(0.0)
    return pstar, qstar, deriv


def graded_raw_probabilities(a, d, theta):
    pstar, qstar, _ = graded_boundary_curves(a, d, theta)
    probs = []
    for k in range(len(pstar) - 1):
        if pstar[k + 1] > 0.5:
            # sisi atas: selisih komplemen, identik secara matematis
            probs.append(qstar[k + 1] - qstar[k])
        else:
            probs.append(pstar[k] - pstar[k + 1])
    return probs


def graded_probabilities(a, d, theta):
    return [max(p, PROB_FLOOR) for p in graded_raw_probabilities(a, d, theta)]


def graded_information(a, d, theta):
    _, _, deriv = graded_boundary_curves(a, d, theta)
    probs = graded_raw_probabilities(a, d, theta)
    n = len(a)
    info = [[0.0] * n for _ in range(n)]
    for k, p_k in enumerate(probs):
        # Suku dengan P_k di bawah lantai (atau gradien persis 0) = 0 (limit).
        if p_k <= PROB_FLOOR:
            continue
        grad_p = [a[j] * (deriv[k] - deriv[k + 1]) for j in range(n)]
        for i in range(n):
            for j in range(n):
                outer = grad_p[i] * grad_p[j]
                if outer != 0.0:
                    info[i][j] += outer / p_k
    return info


def eval_case(nfact, theta_points, prob_fn, info_fn):
    return {
        "nfact": nfact,
        "theta": [list(t) for t in theta_points],
        "probabilities": [prob_fn(t) for t in theta_points],
        "information": [info_fn(t) for t in theta_points],
    }


cases = {}

# 1) M2PL 2D
a, d = [1.2, 0.8], 0.3
theta_pts = [(0.0, 0.0), (1.0, -1.0), (2.0, 2.0)]
cases["m2pl_2d"] = eval_case(
    2, theta_pts,
    lambda t: dich_probabilities(a, d, 0.0, 1.0, t),
    lambda t: dich_information(a, d, 0.0, 1.0, t),
)

# 2) M3PL 2D
a, d, g = [1.0, 1.0], -0.5, 0.2
theta_pts = [(0.0, 0.0)]
cases["m3pl_2d"] = eval_case(
    2, theta_pts,
    lambda t: dich_probabilities(a, d, g, 1.0, t),
    lambda t: dich_information(a, d, g, 1.0, t),
)

# 3) Graded response 2D, 4 kategori
a, d = [1.1, 0.6], [1.5, 0.2, -1.2]
theta_pts = [(-2.0, -2.0), (0.0, 0.0), (2.0, 2.0)]
cases["graded_2d"] = eval_case(
    2, theta_pts,
    lambda t: graded_probabilities(a, d, t),
    lambda t: graded_information(a, d, t),
)

# 4) M2PL 3D
a, d = [0.9, 0.7, 1.1], 0.0
theta_pts = [(0.5, -0.5, 1.0)]
cases["m2pl_3d"] = eval_case(
    3, theta_pts,
    lambda t: dich_probabilities(a, d, 0.0, 1.0, t),
    lambda t: dich_information(a, d, 0.0, 1.0, t),
)

out = {
    "source": "PYTHON_CROSSCHECK -- formula tertutup Reckase (2009)/Barton & Lord (1981)/"
              "Muraki & Carlson (1995) untuk probabilitas kategori & matriks informasi item.",
    "tolerance": 1e-6,
    "cases": cases,
}

print(json.dumps(out, indent=2))
