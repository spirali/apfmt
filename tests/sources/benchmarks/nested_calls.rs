// Adversarial benchmark: deeply nested calls, overflowing last arguments,
// closures inside chains and nested tuples/arrays. Every call can be laid
// out flat, with an overflowing last argument or broken one argument per
// line, so the number of layout alternatives grows with the nesting depth.

fn nested_calls() {
    let result = alpha(beta(gamma(delta(epsilon(zeta(eta(theta(iota(kappa(lambda(mu(nu(xi(omicron(pi(rho(sigma(tau(upsilon(phi(chi(psi(omega(value))))))))))))))))))))))));
    let result = alpha(first_argument, beta(second_argument, gamma(third_argument, delta(fourth_argument, epsilon(fifth_argument, zeta(sixth_argument, eta(seventh_argument, theta(eighth_argument, last_one))))))));
    let result = alpha(beta(aaaaaaaaaa, bbbbbbbbbb), gamma(cccccccccc, dddddddddd), delta(eeeeeeeeee, ffffffffff), epsilon(gggggggggg, hhhhhhhhhh), zeta(iiiiiiiiii, jjjjjjjjjj));
    outer(inner(aaaaaaaaaaaaaaa, bbbbbbbbbbbbbbb), inner(ccccccccccccccc, inner(ddddddddddddddd, inner(eeeeeeeeeeeeeee, inner(fffffffffffffff, ggggggggggggggg)))));
    call(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40);
    call(f(1, 2, 3), f(4, 5, 6), f(7, 8, 9), f(10, 11, 12), f(13, 14, 15), f(16, 17, 18), f(19, 20, 21), f(22, 23, 24), f(25, 26, 27), f(28, 29, 30));
}

fn closures_in_chains() {
    let v = items.iter().map(|x| transform(x, |y| combine(y, |z| finalize(z, aaaaaaaaaaaa, bbbbbbbbbbbb, cccccccccccc)))).filter(|x| predicate(x, dddddddddddd)).collect::<Vec<_>>();
    let v = source.iter().map(|a| a.iter().map(|b| b.iter().map(|c| c.iter().map(|d| d.value + offset).sum::<u64>()).sum::<u64>()).sum::<u64>()).sum::<u64>();
    builder.push(fmt.concat(fmt.text("aaaaaaaaaa"), fmt.nest(fmt.indent, fmt.concat(fmt.text("bbbbbbbbbb"), fmt.alt(fmt.text("cccccccccc"), fmt.concat(fmt.newline(), fmt.text("dddddddddd")))))));
    let x = a.b(|x| c.d(|y| e.f(|z| g.h(|w| i.j(|v| k.l(|u| m.n(|t| o.p(|s| q.r(|r| s.t(|q| q + r + s + t + u + v + w + x + y + z))))))))));
    run(move |ctx| ctx.spawn(move |inner| inner.with(|state| state.update(|value| value.checked_add(increment).unwrap_or_else(|| fallback(value, increment, limit))))));
}

fn nested_tuples_and_arrays() {
    let t = ((aaaaaaaa, bbbbbbbb), (cccccccc, (dddddddd, eeeeeeee)), ((ffffffff, gggggggg), (hhhhhhhh, (iiiiiiii, jjjjjjjj))), kkkkkkkk);
    let a = [[1, 2, 3, 4, 5, 6, 7, 8], [9, 10, 11, 12, 13, 14, 15, 16], [17, 18, 19, 20, 21, 22, 23, 24], [25, 26, 27, 28, 29, 30, 31, 32]];
    let m = [(first(aaaaaaaa), second(bbbbbbbb)), (first(cccccccc), second(dddddddd)), (first(eeeeeeee), second(ffffffff)), (first(gggggggg), second(hhhhhhhh))];
    let deep = f([g((h([i((j([k((l([m((n([o((p([q((r([value]), q))]), o))]), m))]), k))]), i))]), g))]);
}

fn methods_with_arguments() {
    let response = client.request(method, url).header(content_type, application_json).header(authorization, format_token(token, scheme)).body(serialize(payload, options)).timeout(duration(seconds, millis)).send()?.error_for_status()?.json::<Response>()?;
    let total = orders.iter().filter(|order| order.status == Status::Completed && order.date >= start && order.date < end).map(|order| order.items.iter().map(|item| item.price * item.quantity as u64).sum::<u64>()).fold(0, |acc, x| acc + x);
    config.section(name).entry(key).or_insert_with(|| default_value(name, key)).set(compute(previous_value(name, key), increment(step, factor), clamp(minimum, maximum)));
}
