# Task

You are analyzing risk from machine-built decay risk wrappers that include honest_review.
Use only the JSON bundle below. Do not use external knowledge or web search.
Return JSON only, matching honest-review-analysis.v1.
For each wrapper with honest_review, preserve the same mechanism/impact/control statuses or lower them.
Every supported_by item must be copied exactly from honest_review observed or inferred.
Every honest_review unknown must remain visible in limits or uncertainties.
Copy all honest_review overall.non_claims into the output non_claims.
Do not claim a bug, do not prove complete coverage, and do not give a merge recommendation.

# Bundle
```json
{
  "case_id": "hyper-3998",
  "git_context": {
    "base": "743a3ba0706fde95e2095ad42ffefe219d807117",
    "base_input": "743a3ba0706fde95e2095ad42ffefe219d807117",
    "changed_files": 2,
    "head": "4d4e0dfc099d7177721a8dce70fedbea1e6c98e6",
    "repo_root": "/private/tmp/decay-pr-eval/hyper-3998/repo"
  },
  "pr_context": {
    "body": "This is the trailers variant of the fix for reading the body in #3275, so that it is possible to both attempt to read the body and the trailers when the server has sent a `RST_STREAM` with `NO_ERROR` after its response to indicate that the client should stop attempting to send it the body.\r\n\r\nI have added a trailers-only variant of `http2_responds_before_consuming_request_body` that fails without the fix, and also updated `http2_responds_before_consuming_request_body` to verify that it can check whether there are any trailers.",
    "comments": [],
    "files": [
      {
        "additions": 18,
        "changeType": "MODIFIED",
        "deletions": 9,
        "path": "src/body/incoming.rs"
      },
      {
        "additions": 89,
        "changeType": "MODIFIED",
        "deletions": 8,
        "path": "tests/client.rs"
      }
    ],
    "number": 3998,
    "title": "fix(client): reading trailers shouldn't propagate `NO_ERROR` from early response",
    "url": "https://github.com/hyperium/hyper/pull/3998"
  },
  "version": "risk-context-wrapper.v1",
  "wrappers": [
    {
      "analysis_contract": {
        "consumer": "codex-cli",
        "expected_output": "semantic-confidence-evidence.v1",
        "rules": [
          "Use only evidence and context in this wrapper.",
          "Treat primary_evidence as direct evidence.",
          "Treat supporting_evidence as supporting only.",
          "Treat context_evidence as background only.",
          "Do not claim a bug unless the wrapper contains explicit external confirmation.",
          "Mention uncertainty when diff, source, test, or history context is missing."
        ],
        "task": "Produce semantic confidence evidence from this wrapper.",
        "wrapper_type": "ErrorPropagationWrapper"
      },
      "context_evidence": [
        {
          "category": "history",
          "confidence": "low",
          "file": "src/body/incoming.rs",
          "id": 5,
          "kind": "recently_bugfixed_area",
          "source": "git_history",
          "subject": "test(lib): fix unused warnings due to feature gating test imports (#3997)",
          "value": "context"
        }
      ],
      "coverage_limits": [],
      "diff_context": {
        "files": [
          {
            "hunks": [
              {
                "header": "@@ -171,183 +171,192 @@ impl Incoming {",
                "lines": [
                  "     }",
                  " ",
                  "     #[cfg(feature = \"ffi\")]",
                  "     pub(crate) fn as_ffi_mut(&mut self) -> &mut crate::ffi::UserBody {",
                  "         match self.kind {",
                  "             Kind::Ffi(ref mut body) => return body,",
                  "             _ => {",
                  "                 self.kind = Kind::Ffi(crate::ffi::UserBody::new());",
                  "             }",
                  "         }",
                  " ",
                  "         match self.kind {",
                  "             Kind::Ffi(ref mut body) => body,",
                  "             _ => unreachable!(),",
                  "         }",
                  "     }",
                  " }",
                  " ",
                  " impl Body for Incoming {",
                  "     type Data = Bytes;",
                  "     type Error = crate::Error;",
                  " ",
                  "     fn poll_frame(",
                  "         #[cfg_attr(",
                  "             not(all(",
                  "                 any(feature = \"http1\", feature = \"http2\"),",
                  "                 any(feature = \"client\", feature = \"server\")",
                  "             )),",
                  "             allow(unused_mut)",
                  "         )]",
                  "         mut self: Pin<&mut Self>,",
                  "         #[cfg_attr(",
                  "             not(all(",
                  "                 any(feature = \"http1\", feature = \"http2\"),",
                  "                 any(feature = \"client\", feature = \"server\")",
                  "             )),",
                  "             allow(unused_variables)",
                  "         )]",
                  "         cx: &mut Context<'_>,",
                  "     ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {",
                  "         match self.kind {",
                  "             Kind::Empty => Poll::Ready(None),",
                  "             #[cfg(all(feature = \"http1\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::Chan {",
                  "                 content_length: ref mut len,",
                  "                 ref mut data_rx,",
                  "                 ref mut want_tx,",
                  "                 ref mut trailers_rx,",
                  "             } => {",
                  "                 want_tx.send(WANT_READY);",
                  " ",
                  "                 if !data_rx.is_terminated() {",
                  "                     if let Some(chunk) = ready!(Pin::new(data_rx).poll_next(cx)?) {",
                  "                         len.sub_if(chunk.len() as u64);",
                  "                         return Poll::Ready(Some(Ok(Frame::data(chunk))));",
                  "                     }",
                  "                 }",
                  " ",
                  "                 // check trailers after data is terminated",
                  "                 match ready!(Pin::new(trailers_rx).poll(cx)) {",
                  "                     Ok(t) => Poll::Ready(Some(Ok(Frame::trailers(t)))),",
                  "                     Err(_) => Poll::Ready(None),",
                  "                 }",
                  "             }",
                  "             #[cfg(all(feature = \"http2\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::H2 {",
                  "                 ref mut data_done,",
                  "                 ref ping,",
                  "                 recv: ref mut h2,",
                  "                 content_length: ref mut len,",
                  "             } => {",
                  "                 if !*data_done {",
                  "                     match ready!(h2.poll_data(cx)) {",
                  "                         Some(Ok(bytes)) => {",
                  "                             let _ = h2.flow_control().release_capacity(bytes.len());",
                  "                             len.sub_if(bytes.len() as u64);",
                  "                             ping.record_data(bytes.len());",
                  "                             return Poll::Ready(Some(Ok(Frame::data(bytes))));",
                  "                         }",
                  "                         Some(Err(e)) => {",
                  "-                            return match e.reason() {",
                  "-                                // These reasons should cause the body reading to stop, but not fail it.",
                  "-                                // The same logic as for `Read for H2Upgraded` is applied here.",
                  "-                                Some(h2::Reason::NO_ERROR) | Some(h2::Reason::CANCEL) => {",
                  "-                                    Poll::Ready(None)",
                  "-                                }",
                  "-                                _ => Poll::Ready(Some(Err(crate::Error::new_body(e)))),",
                  "-                            };",
                  "+                            if let Some(h2::Reason::NO_ERROR) = e.reason() {",
                  "+                                // As mentioned in RFC 7540 Section 8.1, a RST_STREAM with NO_ERROR",
                  "+                                // indicates an early response, and should cause the body reading",
                  "+                                // to stop, but not fail it:",
                  "+                                return Poll::Ready(None);",
                  "+                            } else {",
                  "+                                return Poll::Ready(Some(Err(crate::Error::new_body(e))));",
                  "+                            }",
                  "                         }",
                  "                         None => {",
                  "                             *data_done = true;",
                  "                             // fall through to trailers",
                  "                         }",
                  "                     }",
                  "                 }",
                  " ",
                  "                 // after data, check trailers",
                  "                 match ready!(h2.poll_trailers(cx)) {",
                  "                     Ok(t) => {",
                  "                         ping.record_non_data();",
                  "                         Poll::Ready(Ok(t.map(Frame::trailers)).transpose())",
                  "                     }",
                  "-                    Err(e) => Poll::Ready(Some(Err(crate::Error::new_h2(e)))),",
                  "+                    Err(e) => {",
                  "+                        if let Some(h2::Reason::NO_ERROR) = e.reason() {",
                  "+                            // Same as above, a RST_STREAM with NO_ERROR indicates an early",
                  "+                            // response, and should cause reading the trailers to stop, but",
                  "+                            // not fail it:",
                  "+                            Poll::Ready(None)",
                  "+                        } else {",
                  "+                            Poll::Ready(Some(Err(crate::Error::new_h2(e))))",
                  "+                        }",
                  "+                    }",
                  "                 }",
                  "             }",
                  " ",
                  "             #[cfg(feature = \"ffi\")]",
                  "             Kind::Ffi(ref mut body) => body.poll_data(cx),",
                  "         }",
                  "     }",
                  " ",
                  "     fn is_end_stream(&self) -> bool {",
                  "         match self.kind {",
                  "             Kind::Empty => true,",
                  "             #[cfg(all(feature = \"http1\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::Chan { content_length, .. } => content_length == DecodedLength::ZERO,",
                  "             #[cfg(all(feature = \"http2\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::H2 { recv: ref h2, .. } => h2.is_end_stream(),",
                  "             #[cfg(feature = \"ffi\")]",
                  "             Kind::Ffi(..) => false,",
                  "         }",
                  "     }",
                  " ",
                  "     fn size_hint(&self) -> SizeHint {",
                  "         #[cfg(all(",
                  "             any(feature = \"http1\", feature = \"http2\"),",
                  "             any(feature = \"client\", feature = \"server\")",
                  "         ))]",
                  "         fn opt_len(decoded_length: DecodedLength) -> SizeHint {",
                  "             if let Some(content_length) = decoded_length.into_opt() {",
                  "                 SizeHint::with_exact(content_length)",
                  "             } else {",
                  "                 SizeHint::default()",
                  "             }",
                  "         }",
                  " ",
                  "         match self.kind {",
                  "             Kind::Empty => SizeHint::with_exact(0),",
                  "             #[cfg(all(feature = \"http1\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::Chan { content_length, .. } => opt_len(content_length),",
                  "             #[cfg(all(feature = \"http2\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::H2 { content_length, .. } => opt_len(content_length),",
                  "             #[cfg(feature = \"ffi\")]",
                  "             Kind::Ffi(..) => SizeHint::default(),",
                  "         }",
                  "     }",
                  " }",
                  " ",
                  " impl fmt::Debug for Incoming {",
                  "     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {",
                  "         #[cfg(any(",
                  "             all(",
                  "                 any(feature = \"http1\", feature = \"http2\"),",
                  "                 any(feature = \"client\", feature = \"server\")",
                  "             ),",
                  "             feature = \"ffi\"",
                  "         ))]",
                  "         #[derive(Debug)]",
                  "         struct Streaming;",
                  "         #[derive(Debug)]",
                  "         struct Empty;",
                  " ",
                  "         let mut builder = f.debug_tuple(\"Body\");",
                  "         match self.kind {",
                  "             Kind::Empty => builder.field(&Empty),",
                  "             #[cfg(any(",
                  "                 all(",
                  "                     any(feature = \"http1\", feature = \"http2\"),",
                  "                     any(feature = \"client\", feature = \"server\")",
                  "                 ),",
                  "                 feature = \"ffi\"",
                  "             ))]",
                  "             _ => builder.field(&Streaming),",
                  "         };",
                  " ",
                  "         builder.finish()",
                  "     }",
                  " }",
                  " ",
                  " #[cfg(all(feature = \"http1\", any(feature = \"client\", feature = \"server\")))]",
                  " impl Sender {",
                  "     /// Check to see if this `Sender` can send more data.",
                  "     pub(crate) fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<crate::Result<()>> {"
                ],
                "reason": "linked_to_primary_evidence"
              }
            ],
            "path": "src/body/incoming.rs"
          }
        ]
      },
      "honest_review": {
        "dimensions": {
          "control": {
            "inferred": [
              "test diff is related to error handling but assertion strength is unclear"
            ],
            "observed": [
              "tests/client.rs: +            type Error = hyper::Error;",
              "tests/client.rs: +            ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {",
              "tests/client.rs: +                        Ok::<_, hyper::Error>(http::Response::new(body))",
              "tests/client.rs: +            conn.await.expect(\"client conn shouldn't error\");",
              "tests/client.rs: +        let (_tx, recv) = mpsc::channel::<Result<Frame<Bytes>, Box<dyn Error + Send + Sync>>>(0);"
            ],
            "status": "weak",
            "unknown": [
              "whether the test exercises the changed path is not proven"
            ]
          },
          "impact": {
            "inferred": [
              "PR metadata is consistent with protocol error or terminal-stream behavior change",
              "impact likely reaches caller-visible body or stream behavior"
            ],
            "observed": [
              "changed production file: src/body/incoming.rs",
              "PR title: fix(client): reading trailers shouldn't propagate `NO_ERROR` from early response",
              "PR body: This is the trailers variant of the fix for reading the body in #3275, so that it is possible to both attempt to read the body and the trailers when the server has sent a `RST_STREAM` with `NO_ERROR` after its response to indicate that the client should stop attempting to send it the body.\r\n\r\nI have added a trailers-only variant of `http2_responds_before_consuming_request_body` that fails without the fix, and also updated `http2_responds_before_consuming_request_body` to verify that it can check whether there are any trailers.",
              "related test diff calls a public entry point or asserts an error"
            ],
            "status": "partial",
            "unknown": [
              "full public API contract is not available in this wrapper",
              "call graph from changed function to every caller is not available in this wrapper",
              "whether every affected entry point is covered is not proven"
            ]
          },
          "mechanism": {
            "inferred": [
              "error propagation behavior changed in the same diff hunk"
            ],
            "observed": [
              "added explicit error return: code contains `return Poll::Ready(None)`",
              "removed error-flow behavior: code contains `Some(h2::Reason::NO_ERROR) | Some(h2::Reason::CANCEL) => {`",
              "added error-flow behavior: code contains `if let Some(h2::Reason::NO_ERROR) = e.reason() {`"
            ],
            "status": "partial",
            "unknown": [
              "whether this changes caller-visible behavior is not proven by the wrapper alone",
              "whether this is a fix, compatibility change, or risk introduction is not proven"
            ]
          }
        },
        "overall": {
          "control_status": "weak",
          "impact_status": "partial",
          "judgement": "needs_more_context_before_review_claim",
          "mechanism_status": "partial",
          "non_claims": [
            "not evidence that this PR introduced a bug",
            "not proof of complete test coverage",
            "not a merge recommendation"
          ]
        },
        "risk_item": "error_propagation_change",
        "version": "honest-review.v1"
      },
      "primary_evidence": [
        {
          "category": "semantic",
          "confidence": "medium",
          "file": "src/body/incoming.rs",
          "id": 3,
          "kind": "error_propagation_changed",
          "source": "semantic_diff",
          "subject": "return Poll::Ready(None);",
          "value": "direct"
        }
      ],
      "risk_question": "Did this diff change error propagation or terminal error handling?",
      "supporting_evidence": [
        {
          "category": "quality",
          "confidence": "low",
          "file": "src/body/incoming.rs",
          "id": 2,
          "kind": "prod_change_with_tests",
          "source": "quality",
          "subject": "production code changed with test changes",
          "value": "supporting"
        }
      ],
      "wrapper_context": {
        "after_error_flow_lines": [
          "+                            if let Some(h2::Reason::NO_ERROR) = e.reason() {",
          "+                                // As mentioned in RFC 7540 Section 8.1, a RST_STREAM with NO_ERROR",
          "+                                return Poll::Ready(None);",
          "+                                return Poll::Ready(Some(Err(crate::Error::new_body(e))));",
          "+                    Err(e) => {",
          "+                        if let Some(h2::Reason::NO_ERROR) = e.reason() {",
          "+                            // Same as above, a RST_STREAM with NO_ERROR indicates an early",
          "+                            Poll::Ready(None)",
          "+                            Poll::Ready(Some(Err(crate::Error::new_h2(e))))"
        ],
        "after_error_lines": [
          "+                                return Poll::Ready(None);",
          "+                                return Poll::Ready(Some(Err(crate::Error::new_body(e))));",
          "+                    Err(e) => {",
          "+                            Poll::Ready(Some(Err(crate::Error::new_h2(e))))"
        ],
        "before_error_flow_lines": [
          "-                                Some(h2::Reason::NO_ERROR) | Some(h2::Reason::CANCEL) => {",
          "-                                    Poll::Ready(None)",
          "-                                _ => Poll::Ready(Some(Err(crate::Error::new_body(e)))),",
          "-                    Err(e) => Poll::Ready(Some(Err(crate::Error::new_h2(e)))),"
        ],
        "before_panic_lines": [],
        "error_tokens": [],
        "primary_kind": "error_propagation_changed",
        "related_test_lines": [
          "tests/client.rs: +            type Error = hyper::Error;",
          "tests/client.rs: +            ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {",
          "tests/client.rs: +                        Ok::<_, hyper::Error>(http::Response::new(body))",
          "tests/client.rs: +            conn.await.expect(\"client conn shouldn't error\");",
          "tests/client.rs: +        let (_tx, recv) = mpsc::channel::<Result<Frame<Bytes>, Box<dyn Error + Send + Sync>>>(0);"
        ],
        "shared_input_tokens": []
      },
      "wrapper_id": "w1",
      "wrapper_type": "ErrorPropagationWrapper"
    },
    {
      "analysis_contract": {
        "consumer": "codex-cli",
        "expected_output": "semantic-confidence-evidence.v1",
        "rules": [
          "Use only evidence and context in this wrapper.",
          "Treat primary_evidence as direct evidence.",
          "Treat supporting_evidence as supporting only.",
          "Treat context_evidence as background only.",
          "Do not claim a bug unless the wrapper contains explicit external confirmation.",
          "Mention uncertainty when diff, source, test, or history context is missing."
        ],
        "task": "Produce semantic confidence evidence from this wrapper.",
        "wrapper_type": "ErrorPropagationWrapper"
      },
      "context_evidence": [
        {
          "category": "history",
          "confidence": "low",
          "file": "src/body/incoming.rs",
          "id": 5,
          "kind": "recently_bugfixed_area",
          "source": "git_history",
          "subject": "test(lib): fix unused warnings due to feature gating test imports (#3997)",
          "value": "context"
        }
      ],
      "coverage_limits": [],
      "diff_context": {
        "files": [
          {
            "hunks": [
              {
                "header": "@@ -171,183 +171,192 @@ impl Incoming {",
                "lines": [
                  "     }",
                  " ",
                  "     #[cfg(feature = \"ffi\")]",
                  "     pub(crate) fn as_ffi_mut(&mut self) -> &mut crate::ffi::UserBody {",
                  "         match self.kind {",
                  "             Kind::Ffi(ref mut body) => return body,",
                  "             _ => {",
                  "                 self.kind = Kind::Ffi(crate::ffi::UserBody::new());",
                  "             }",
                  "         }",
                  " ",
                  "         match self.kind {",
                  "             Kind::Ffi(ref mut body) => body,",
                  "             _ => unreachable!(),",
                  "         }",
                  "     }",
                  " }",
                  " ",
                  " impl Body for Incoming {",
                  "     type Data = Bytes;",
                  "     type Error = crate::Error;",
                  " ",
                  "     fn poll_frame(",
                  "         #[cfg_attr(",
                  "             not(all(",
                  "                 any(feature = \"http1\", feature = \"http2\"),",
                  "                 any(feature = \"client\", feature = \"server\")",
                  "             )),",
                  "             allow(unused_mut)",
                  "         )]",
                  "         mut self: Pin<&mut Self>,",
                  "         #[cfg_attr(",
                  "             not(all(",
                  "                 any(feature = \"http1\", feature = \"http2\"),",
                  "                 any(feature = \"client\", feature = \"server\")",
                  "             )),",
                  "             allow(unused_variables)",
                  "         )]",
                  "         cx: &mut Context<'_>,",
                  "     ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {",
                  "         match self.kind {",
                  "             Kind::Empty => Poll::Ready(None),",
                  "             #[cfg(all(feature = \"http1\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::Chan {",
                  "                 content_length: ref mut len,",
                  "                 ref mut data_rx,",
                  "                 ref mut want_tx,",
                  "                 ref mut trailers_rx,",
                  "             } => {",
                  "                 want_tx.send(WANT_READY);",
                  " ",
                  "                 if !data_rx.is_terminated() {",
                  "                     if let Some(chunk) = ready!(Pin::new(data_rx).poll_next(cx)?) {",
                  "                         len.sub_if(chunk.len() as u64);",
                  "                         return Poll::Ready(Some(Ok(Frame::data(chunk))));",
                  "                     }",
                  "                 }",
                  " ",
                  "                 // check trailers after data is terminated",
                  "                 match ready!(Pin::new(trailers_rx).poll(cx)) {",
                  "                     Ok(t) => Poll::Ready(Some(Ok(Frame::trailers(t)))),",
                  "                     Err(_) => Poll::Ready(None),",
                  "                 }",
                  "             }",
                  "             #[cfg(all(feature = \"http2\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::H2 {",
                  "                 ref mut data_done,",
                  "                 ref ping,",
                  "                 recv: ref mut h2,",
                  "                 content_length: ref mut len,",
                  "             } => {",
                  "                 if !*data_done {",
                  "                     match ready!(h2.poll_data(cx)) {",
                  "                         Some(Ok(bytes)) => {",
                  "                             let _ = h2.flow_control().release_capacity(bytes.len());",
                  "                             len.sub_if(bytes.len() as u64);",
                  "                             ping.record_data(bytes.len());",
                  "                             return Poll::Ready(Some(Ok(Frame::data(bytes))));",
                  "                         }",
                  "                         Some(Err(e)) => {",
                  "-                            return match e.reason() {",
                  "-                                // These reasons should cause the body reading to stop, but not fail it.",
                  "-                                // The same logic as for `Read for H2Upgraded` is applied here.",
                  "-                                Some(h2::Reason::NO_ERROR) | Some(h2::Reason::CANCEL) => {",
                  "-                                    Poll::Ready(None)",
                  "-                                }",
                  "-                                _ => Poll::Ready(Some(Err(crate::Error::new_body(e)))),",
                  "-                            };",
                  "+                            if let Some(h2::Reason::NO_ERROR) = e.reason() {",
                  "+                                // As mentioned in RFC 7540 Section 8.1, a RST_STREAM with NO_ERROR",
                  "+                                // indicates an early response, and should cause the body reading",
                  "+                                // to stop, but not fail it:",
                  "+                                return Poll::Ready(None);",
                  "+                            } else {",
                  "+                                return Poll::Ready(Some(Err(crate::Error::new_body(e))));",
                  "+                            }",
                  "                         }",
                  "                         None => {",
                  "                             *data_done = true;",
                  "                             // fall through to trailers",
                  "                         }",
                  "                     }",
                  "                 }",
                  " ",
                  "                 // after data, check trailers",
                  "                 match ready!(h2.poll_trailers(cx)) {",
                  "                     Ok(t) => {",
                  "                         ping.record_non_data();",
                  "                         Poll::Ready(Ok(t.map(Frame::trailers)).transpose())",
                  "                     }",
                  "-                    Err(e) => Poll::Ready(Some(Err(crate::Error::new_h2(e)))),",
                  "+                    Err(e) => {",
                  "+                        if let Some(h2::Reason::NO_ERROR) = e.reason() {",
                  "+                            // Same as above, a RST_STREAM with NO_ERROR indicates an early",
                  "+                            // response, and should cause reading the trailers to stop, but",
                  "+                            // not fail it:",
                  "+                            Poll::Ready(None)",
                  "+                        } else {",
                  "+                            Poll::Ready(Some(Err(crate::Error::new_h2(e))))",
                  "+                        }",
                  "+                    }",
                  "                 }",
                  "             }",
                  " ",
                  "             #[cfg(feature = \"ffi\")]",
                  "             Kind::Ffi(ref mut body) => body.poll_data(cx),",
                  "         }",
                  "     }",
                  " ",
                  "     fn is_end_stream(&self) -> bool {",
                  "         match self.kind {",
                  "             Kind::Empty => true,",
                  "             #[cfg(all(feature = \"http1\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::Chan { content_length, .. } => content_length == DecodedLength::ZERO,",
                  "             #[cfg(all(feature = \"http2\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::H2 { recv: ref h2, .. } => h2.is_end_stream(),",
                  "             #[cfg(feature = \"ffi\")]",
                  "             Kind::Ffi(..) => false,",
                  "         }",
                  "     }",
                  " ",
                  "     fn size_hint(&self) -> SizeHint {",
                  "         #[cfg(all(",
                  "             any(feature = \"http1\", feature = \"http2\"),",
                  "             any(feature = \"client\", feature = \"server\")",
                  "         ))]",
                  "         fn opt_len(decoded_length: DecodedLength) -> SizeHint {",
                  "             if let Some(content_length) = decoded_length.into_opt() {",
                  "                 SizeHint::with_exact(content_length)",
                  "             } else {",
                  "                 SizeHint::default()",
                  "             }",
                  "         }",
                  " ",
                  "         match self.kind {",
                  "             Kind::Empty => SizeHint::with_exact(0),",
                  "             #[cfg(all(feature = \"http1\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::Chan { content_length, .. } => opt_len(content_length),",
                  "             #[cfg(all(feature = \"http2\", any(feature = \"client\", feature = \"server\")))]",
                  "             Kind::H2 { content_length, .. } => opt_len(content_length),",
                  "             #[cfg(feature = \"ffi\")]",
                  "             Kind::Ffi(..) => SizeHint::default(),",
                  "         }",
                  "     }",
                  " }",
                  " ",
                  " impl fmt::Debug for Incoming {",
                  "     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {",
                  "         #[cfg(any(",
                  "             all(",
                  "                 any(feature = \"http1\", feature = \"http2\"),",
                  "                 any(feature = \"client\", feature = \"server\")",
                  "             ),",
                  "             feature = \"ffi\"",
                  "         ))]",
                  "         #[derive(Debug)]",
                  "         struct Streaming;",
                  "         #[derive(Debug)]",
                  "         struct Empty;",
                  " ",
                  "         let mut builder = f.debug_tuple(\"Body\");",
                  "         match self.kind {",
                  "             Kind::Empty => builder.field(&Empty),",
                  "             #[cfg(any(",
                  "                 all(",
                  "                     any(feature = \"http1\", feature = \"http2\"),",
                  "                     any(feature = \"client\", feature = \"server\")",
                  "                 ),",
                  "                 feature = \"ffi\"",
                  "             ))]",
                  "             _ => builder.field(&Streaming),",
                  "         };",
                  " ",
                  "         builder.finish()",
                  "     }",
                  " }",
                  " ",
                  " #[cfg(all(feature = \"http1\", any(feature = \"client\", feature = \"server\")))]",
                  " impl Sender {",
                  "     /// Check to see if this `Sender` can send more data.",
                  "     pub(crate) fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<crate::Result<()>> {"
                ],
                "reason": "linked_to_primary_evidence"
              }
            ],
            "path": "src/body/incoming.rs"
          }
        ]
      },
      "honest_review": {
        "dimensions": {
          "control": {
            "inferred": [
              "test diff is related to error handling but assertion strength is unclear"
            ],
            "observed": [
              "tests/client.rs: +            type Error = hyper::Error;",
              "tests/client.rs: +            ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {",
              "tests/client.rs: +                        Ok::<_, hyper::Error>(http::Response::new(body))",
              "tests/client.rs: +            conn.await.expect(\"client conn shouldn't error\");",
              "tests/client.rs: +        let (_tx, recv) = mpsc::channel::<Result<Frame<Bytes>, Box<dyn Error + Send + Sync>>>(0);"
            ],
            "status": "weak",
            "unknown": [
              "whether the test exercises the changed path is not proven"
            ]
          },
          "impact": {
            "inferred": [
              "PR metadata is consistent with protocol error or terminal-stream behavior change",
              "impact likely reaches caller-visible body or stream behavior"
            ],
            "observed": [
              "changed production file: src/body/incoming.rs",
              "PR title: fix(client): reading trailers shouldn't propagate `NO_ERROR` from early response",
              "PR body: This is the trailers variant of the fix for reading the body in #3275, so that it is possible to both attempt to read the body and the trailers when the server has sent a `RST_STREAM` with `NO_ERROR` after its response to indicate that the client should stop attempting to send it the body.\r\n\r\nI have added a trailers-only variant of `http2_responds_before_consuming_request_body` that fails without the fix, and also updated `http2_responds_before_consuming_request_body` to verify that it can check whether there are any trailers.",
              "related test diff calls a public entry point or asserts an error"
            ],
            "status": "partial",
            "unknown": [
              "full public API contract is not available in this wrapper",
              "call graph from changed function to every caller is not available in this wrapper",
              "whether every affected entry point is covered is not proven"
            ]
          },
          "mechanism": {
            "inferred": [
              "error propagation behavior changed in the same diff hunk"
            ],
            "observed": [
              "added explicit error return: code contains `return Poll::Ready(None)`",
              "removed error-flow behavior: code contains `Some(h2::Reason::NO_ERROR) | Some(h2::Reason::CANCEL) => {`",
              "added error-flow behavior: code contains `if let Some(h2::Reason::NO_ERROR) = e.reason() {`"
            ],
            "status": "partial",
            "unknown": [
              "whether this changes caller-visible behavior is not proven by the wrapper alone",
              "whether this is a fix, compatibility change, or risk introduction is not proven"
            ]
          }
        },
        "overall": {
          "control_status": "weak",
          "impact_status": "partial",
          "judgement": "needs_more_context_before_review_claim",
          "mechanism_status": "partial",
          "non_claims": [
            "not evidence that this PR introduced a bug",
            "not proof of complete test coverage",
            "not a merge recommendation"
          ]
        },
        "risk_item": "error_propagation_change",
        "version": "honest-review.v1"
      },
      "primary_evidence": [
        {
          "category": "semantic",
          "confidence": "medium",
          "file": "src/body/incoming.rs",
          "id": 4,
          "kind": "error_propagation_changed",
          "source": "semantic_diff",
          "subject": "Poll::Ready(None)",
          "value": "direct"
        }
      ],
      "risk_question": "Did this diff change error propagation or terminal error handling?",
      "supporting_evidence": [
        {
          "category": "quality",
          "confidence": "low",
          "file": "src/body/incoming.rs",
          "id": 2,
          "kind": "prod_change_with_tests",
          "source": "quality",
          "subject": "production code changed with test changes",
          "value": "supporting"
        }
      ],
      "wrapper_context": {
        "after_error_flow_lines": [
          "+                            if let Some(h2::Reason::NO_ERROR) = e.reason() {",
          "+                                // As mentioned in RFC 7540 Section 8.1, a RST_STREAM with NO_ERROR",
          "+                                return Poll::Ready(None);",
          "+                                return Poll::Ready(Some(Err(crate::Error::new_body(e))));",
          "+                    Err(e) => {",
          "+                        if let Some(h2::Reason::NO_ERROR) = e.reason() {",
          "+                            // Same as above, a RST_STREAM with NO_ERROR indicates an early",
          "+                            Poll::Ready(None)",
          "+                            Poll::Ready(Some(Err(crate::Error::new_h2(e))))"
        ],
        "after_error_lines": [
          "+                                return Poll::Ready(None);",
          "+                                return Poll::Ready(Some(Err(crate::Error::new_body(e))));",
          "+                    Err(e) => {",
          "+                            Poll::Ready(Some(Err(crate::Error::new_h2(e))))"
        ],
        "before_error_flow_lines": [
          "-                                Some(h2::Reason::NO_ERROR) | Some(h2::Reason::CANCEL) => {",
          "-                                    Poll::Ready(None)",
          "-                                _ => Poll::Ready(Some(Err(crate::Error::new_body(e)))),",
          "-                    Err(e) => Poll::Ready(Some(Err(crate::Error::new_h2(e)))),"
        ],
        "before_panic_lines": [],
        "error_tokens": [],
        "primary_kind": "error_propagation_changed",
        "related_test_lines": [
          "tests/client.rs: +            type Error = hyper::Error;",
          "tests/client.rs: +            ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {",
          "tests/client.rs: +                        Ok::<_, hyper::Error>(http::Response::new(body))",
          "tests/client.rs: +            conn.await.expect(\"client conn shouldn't error\");",
          "tests/client.rs: +        let (_tx, recv) = mpsc::channel::<Result<Frame<Bytes>, Box<dyn Error + Send + Sync>>>(0);"
        ],
        "shared_input_tokens": []
      },
      "wrapper_id": "w2",
      "wrapper_type": "ErrorPropagationWrapper"
    }
  ]
}
```
