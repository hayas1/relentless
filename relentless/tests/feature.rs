use relentless::{
    report::{ReportFormat, Reporter},
    shot::job::{Job, JobSpec},
    testing::TestingClient,
};

#[tokio::test]
async fn test_pass() {
    let spec =
        JobSpec { report_format: ReportFormat::Console, base_path: Some("..".parse().unwrap()), ..Default::default() };
    for expect in ["pass", "allow"] {
        let files: Result<Vec<_>, _> = glob::glob(&format!("tests/config/{expect}/*.yaml")).unwrap().collect();
        let job = Job::from_files(&files.unwrap()).unwrap();

        let make = TestingClient;
        let report = job.shot::<TestingClient, TestingClient, TestingClient>(make, &spec).await.unwrap();
        spec.report(&report);

        match expect {
            "pass" => assert!(report.evaluated.pass && report.evaluated.allow),
            "allow" => assert!(!report.evaluated.pass && report.evaluated.allow),
            _ => unreachable!(),
        }
    }
}
#[tokio::test]
async fn test_fail() {
    let spec =
        JobSpec { report_format: ReportFormat::Console, base_path: Some("..".parse().unwrap()), ..Default::default() };
    let expect = "fail";
    for path in glob::glob(&format!("tests/config/{expect}/*.yaml")).unwrap() {
        let job = Job::from_files(&[path.unwrap()]).unwrap();

        let make = TestingClient;
        let report = job.shot::<TestingClient, TestingClient, TestingClient>(make, &spec).await.unwrap();
        spec.report(&report);

        assert!(!report.evaluated.pass && !report.evaluated.allow)
    }
}
