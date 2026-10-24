from bpmninja_crew.models import Recommendation, RecommendationReport, ScoreBreakdown


def make_recommendation(title: str, impact: int) -> Recommendation:
    return Recommendation(
        title=title,
        category="engine",
        summary="Test",
        proposed_change="Test",
        acceptance_criteria=["Test läuft"],
        score_breakdown=ScoreBreakdown(
            impact=impact,
            domain_gap=10,
            risk_reduction=10,
            effort=10,
            evidence=10,
        ),
    )


def test_score_is_sum_of_breakdown():
    rec = make_recommendation("A", 30)
    assert rec.score == 70


def test_report_sorts_and_renumbers():
    report = RecommendationReport(
        recommendations=[
            make_recommendation("klein", 5),
            make_recommendation("groß", 30),
        ]
    )
    assert [rec.id for rec in report.recommendations] == ["R1", "R2"]
    assert report.recommendations[0].title == "groß"
    assert report.recommendations[0].score == 70
