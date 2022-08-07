import http from "k6/http";
import { check } from "k6";

export function setup() {
  const tokens = [];
  for (let n = 0; n < 1000; n++) {
    let r = http.post(
      "http://localhost:8080/admin/tokens", 
      JSON.stringify({target: "http://example.com"}), 
      {headers: {"Content-Type": "application/json"}}
    );
    check(r, { "status is 201": (r) => r.status == 201});
    const token = r.json("token");
    tokens.push(token);
  }
  return tokens;
}

export default function (tokens) {
  const token = tokens[Math.floor(Math.random()*tokens.length)];
  let r = http.get("http://localhost:8080/" + token, {redirects: 0});
  check(r, {
    "status is 307": (r) => r.status === 307,
  });
}
