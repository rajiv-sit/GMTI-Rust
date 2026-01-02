#include "DataProvider.h"

#include <QCoreApplication>
#include <QDir>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QTimer>
#include <QUrl>

struct DataProvider::Impl
{
    QTimer timer;
    QNetworkAccessManager manager;
};

DataProvider::DataProvider(QObject* parent)
    : QObject(parent)
    , d(new Impl{QTimer(parent), QNetworkAccessManager(parent)})
{
    connect(&d->timer, &QTimer::timeout, this, &DataProvider::refresh);
}

DataProvider::~DataProvider()
{
    delete d;
}

void DataProvider::start(int interval_ms)
{
    d->timer.setInterval(interval_ms);
    refresh();
    d->timer.start();
}

void DataProvider::refresh()
{
    QNetworkRequest request(QUrl("http://127.0.0.1:9000/payload"));
    auto* reply = d->manager.get(request);
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        reply->deleteLater();
        if (reply->error() != QNetworkReply::NoError) {
            return;
        }

        const auto doc = QJsonDocument::fromJson(reply->readAll());
        if (!doc.isObject()) {
            return;
        }

        const auto obj = doc.object();
        const auto powerArray = obj.value("power_profile").toArray();
        QVector<double> profile;
        profile.reserve(powerArray.size());
        for (const auto& value : powerArray) {
            profile.append(value.toDouble());
        }

        const int detections = obj.value("detection_count").toInt(0);
        emit dataReady(profile, detections);
    });
}
