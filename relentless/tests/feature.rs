use relentless::{
    report::ReportFormat,
    shot::job::{Job, JobSpec},
    testing::TestingClient,
};

#[tokio::test]
async fn test_feature() {
    let spec =
        JobSpec { report_format: ReportFormat::Console, base_path: Some("..".parse().unwrap()), ..Default::default() };
    for expect in ["pass", "allow", "fail"] {
        let files: Result<Vec<_>, _> = glob::glob(&format!("tests/config/{expect}/*.yaml")).unwrap().collect();
        let job = Job::from_files(&files.unwrap()).unwrap();

        let make = TestingClient;
        let report = job.shot::<TestingClient, TestingClient, TestingClient>(make, &spec).await.unwrap();

        match expect {
            "pass" => assert!(report.evaluated.pass && report.evaluated.allow),
            "allow" => assert!(!report.evaluated.pass && report.evaluated.allow),
            "fail" => assert!(!report.evaluated.pass && !report.evaluated.allow),
            _ => unreachable!(),
        }
    }
}
